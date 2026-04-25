//! Run a full analysis pass on the current source set and publish the
//! resulting diagnostics, grouped per file.
//!
//! Called from `didOpen`/`didChange`/`didClose`. The actual compiler work
//! (`infer_all`, `analyze_all`) runs on a `spawn_blocking` worker so the
//! tokio IO thread stays responsive.

use std::collections::HashMap;

use kestrel_compiler_driver::CompilerDriver;
use tower_lsp::Client;
use tower_lsp::lsp_types::{Diagnostic as LspDiagnostic, Url};

use crate::convert::{from_analyze, from_codespan, FileMap};
use crate::position::LineIndex;
use crate::server::{path_to_url, rebuild_compiler, SharedState};

/// Reanalyze + publish. Idempotent — safe to call from any handler.
pub async fn refresh(state: SharedState, client: Client) {
    // Snapshot inputs while the lock is held briefly.
    let (token_at_start, sources, doc_indices, disk_indices, prev_published) = {
        let s = state.lock().await;
        let sources = s.sources.clone();
        let doc_indices: HashMap<String, LineIndex> = s
            .docs
            .iter()
            .map(|(uri, doc)| (super::super::server::url_to_path(uri), doc.line_index.clone()))
            .collect();
        let disk = s.disk_line_indices.clone();
        let pub_set = s.published.clone();
        (s.revision_token, sources, doc_indices, disk, pub_set)
    };

    // Heavy work on a blocking worker.
    let analysis = tokio::task::spawn_blocking(move || {
        let (compiler, _by_path) = rebuild_compiler(&sources);
        let driver = CompilerDriver::new(&compiler);
        // infer_all populates per-body diagnostics; analyze_all returns
        // its own diagnostic vector and also accumulates into the world.
        let _infer = driver.infer_all();
        let analyze = driver.analyze_all();
        let codespan_diags = compiler.diagnostics();
        let id_to_path: HashMap<usize, String> = compiler
            .files()
            .iter()
            .map(|(p, e)| (e.index(), p.clone()))
            .collect();
        (codespan_diags, analyze.diagnostics, id_to_path)
    })
    .await
    .expect("analysis worker panicked");

    let (codespan_diags, analyze_diags, id_to_path) = analysis;

    // If another edit landed while we were computing, drop our results.
    {
        let s = state.lock().await;
        if s.revision_token != token_at_start {
            return;
        }
    }

    // Build the FileMap: file_id → (Url, &LineIndex). Prefer the open-doc
    // line index; fall back to the disk index for files not currently open.
    let mut by_id: HashMap<usize, (Url, &LineIndex)> = HashMap::new();
    for (id, path) in &id_to_path {
        let Some(url) = path_to_url(path) else { continue };
        let idx = doc_indices.get(path).or_else(|| disk_indices.get(path));
        if let Some(idx) = idx {
            by_id.insert(*id, (url, idx));
        }
    }
    let files = FileMap { by_id };

    // Group diagnostics by URL.
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
