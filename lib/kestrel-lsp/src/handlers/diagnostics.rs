//! Run a full analysis pass on the current source set and publish the
//! resulting diagnostics, grouped per file.
//!
//! Called from `didOpen`/`didChange`/`didClose`. The actual compiler work
//! (`infer_all`, `analyze_all`) runs against the persistent `Compiler`
//! owned by [`crate::compiler_worker`]; on cache hits this is essentially
//! free.
//!
//! Line indices are built **here, per pass, for the handful of files a
//! diagnostic actually points at** — never cached in `ServerState`. A
//! server-lifetime `path → LineIndex` map is a second copy of text that
//! `sources` already owns, and any writer that forgets to update it
//! publishes ranges computed against stale text (F38: `didClose` on an
//! edited buffer misplaced every squiggle in the file). `sources` is the
//! single source of truth for file text; indices are derived from it.

use std::collections::{HashMap, HashSet};

use codespan_reporting::diagnostic::Diagnostic as CsDiagnostic;
use kestrel_analyze::AnalyzeDiagnostic;
use kestrel_compiler_driver::CompilerDriver;
use tower_lsp::Client;
use tower_lsp::lsp_types::{Diagnostic as LspDiagnostic, MessageType, Url};

use crate::convert::{FileMap, from_analyze, from_codespan};
use crate::position::LineIndex;
use crate::server::{SharedState, path_to_url, url_to_path};

/// Every file id any label in either diagnostic stream points at.
///
/// `FileMap::lookup` is only ever reached from a label's `file_id`
/// (`from_codespan`'s primary + related labels, `from_analyze` via
/// `label_range` / `span_to_location`), so this is exactly the set of
/// indices we need to resolve — usually 0-5, versus every compiled file.
fn referenced_file_ids(
    codespan: &[CsDiagnostic<usize>],
    analyze: &[AnalyzeDiagnostic],
) -> HashSet<usize> {
    let mut ids = HashSet::new();
    for diag in codespan {
        ids.extend(diag.labels.iter().map(|l| l.file_id));
    }
    for diag in analyze {
        ids.extend(diag.labels.iter().map(|l| l.span.file_id));
    }
    ids
}

/// Reanalyze + publish. Idempotent — safe to call from any handler.
pub async fn refresh(state: SharedState, client: Client) {
    // Snapshot inputs while the lock is held briefly.
    let (token_at_start, handle, stdlib, user, prev_published) = {
        let s = state.lock().await;
        let (stdlib, user) = s.partition_sources();
        (
            s.revision_token,
            s.compiler_handle.clone(),
            stdlib,
            user,
            s.published.clone(),
        )
    };

    // Heavy work runs on the worker thread. With the persistent
    // compiler, repeated refreshes after the first sync are cache hits.
    let Some(analysis) = handle
        .with_compiler(stdlib, user, |compiler, _by_path| {
            let driver = CompilerDriver::new(compiler);
            // infer_all populates per-body diagnostics; analyze_all returns
            // its own diagnostic vector and also accumulates into the world.
            let _infer = driver.infer_all();
            let analyze = driver.analyze_all(false);
            let codespan_diags = compiler.diagnostics();
            // Resolve paths only for the files a diagnostic mentions.
            let needed = referenced_file_ids(&codespan_diags, &analyze.diagnostics);
            let id_to_path: HashMap<usize, String> = compiler
                .files()
                .iter()
                .filter(|(_, e)| needed.contains(&e.index()))
                .map(|(p, e)| (e.index(), p.clone()))
                .collect();
            (codespan_diags, analyze.diagnostics, id_to_path, needed)
        })
        .await
    else {
        return;
    };

    let (codespan_diags, analyze_diags, id_to_path, needed) = analysis;

    // Build the owning index map: file_id → (Url, LineIndex). Open buffers
    // win (their text is the one the editor is showing); everything else
    // derives from `sources`, which holds the right text for disk-loaded
    // files, closed-but-unsaved buffers, and files the workspace walk never
    // visited alike. `owned` outlives `files` on this stack, so `FileMap`
    // can keep borrowing.
    let mut owned: HashMap<usize, (Url, LineIndex)> = HashMap::new();
    let mut unresolved: Vec<String> = Vec::new();
    {
        let s = state.lock().await;
        // If another edit landed while we were computing, drop our results.
        if s.revision_token != token_at_start {
            return;
        }
        let doc_indices: HashMap<String, &LineIndex> = s
            .docs
            .iter()
            .map(|(uri, doc)| (url_to_path(uri), &doc.line_index))
            .collect();
        for id in &needed {
            let Some(path) = id_to_path.get(id) else {
                unresolved.push(format!("<file id {id}>"));
                continue;
            };
            let Some(url) = path_to_url(path) else {
                unresolved.push(path.clone());
                continue;
            };
            if let Some(idx) = doc_indices.get(path) {
                owned.insert(*id, (url, (*idx).clone()));
            } else if let Some(text) = s.sources.get(path) {
                owned.insert(*id, (url, LineIndex::new(text.clone())));
            } else {
                unresolved.push(path.clone());
            }
        }
    }

    // A file can only carry a diagnostic if it was compiled, which means it
    // was in `sources` — so this should be unreachable. Say so out loud
    // rather than silently dropping the diagnostic, which is how the same
    // class of bug hid before (`convert.rs`'s `?` chains have no idea a
    // lookup was supposed to succeed).
    if !unresolved.is_empty() {
        unresolved.sort();
        unresolved.dedup();
        client
            .log_message(
                MessageType::WARNING,
                format!(
                    "Kestrel: dropped diagnostics for {} file(s) with no resolvable source text: {}",
                    unresolved.len(),
                    unresolved.join(", ")
                ),
            )
            .await;
    }

    let files = FileMap {
        by_id: owned
            .iter()
            .map(|(id, (url, idx))| (*id, (url.clone(), idx)))
            .collect(),
    };

    // Group diagnostics by URL. The two streams are disjoint by construction:
    // inference errors are rendered only by the codespan stream (F15), so
    // pushing both here cannot produce the double squiggle it used to.
    let mut grouped: HashMap<Url, Vec<LspDiagnostic>> = HashMap::new();
    for diag in &codespan_diags {
        if let Some((file_id, lsp_diag)) = from_codespan(diag, &files)
            && let Some((url, _)) = files.lookup(file_id)
        {
            grouped.entry(url.clone()).or_default().push(lsp_diag);
        }
    }
    for diag in &analyze_diags {
        if let Some((file_id, lsp_diag)) = from_analyze(diag, &files)
            && let Some((url, _)) = files.lookup(file_id)
        {
            grouped.entry(url.clone()).or_default().push(lsp_diag);
        }
    }

    // Send every URL: known files with diagnostics, plus previously
    // published URLs that now have none (clear stale squiggles).
    let mut to_publish: HashMap<Url, Vec<LspDiagnostic>> = grouped;
    for url in &prev_published {
        to_publish.entry(url.clone()).or_default();
    }

    // Track newly-published URLs.
    {
        let mut s = state.lock().await;
        for url in to_publish.keys() {
            s.published.insert(url.clone());
        }
    }

    for (url, diags) in to_publish {
        client.publish_diagnostics(url, diags, None).await;
    }
}
