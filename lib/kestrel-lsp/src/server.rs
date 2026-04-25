//! Server state shared between request handlers.
//!
//! M1 strategy: keep a canonical `sources: HashMap<path, text>` map and
//! rebuild a fresh `Compiler` on every analysis pass. The compiler crate's
//! AST builder is not idempotent — calling `build()` twice on a file
//! creates duplicate declaration entities — so we can't reuse a Compiler
//! across edits. Per-pass cost is dominated by `infer_all` over the loaded
//! sources; that's tolerable for small projects in M1 and an explicit
//! optimization target for M5 (after the AST builder gains rebuild support).

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use kestrel_compiler::Compiler;
use kestrel_hecs::Entity;
use tokio::sync::Mutex;
use tower_lsp::lsp_types::Url;

use crate::documents::OpenDocs;
use crate::position::LineIndex;

pub struct ServerState {
    /// Compiler-key (canonical path) → source text. Single source of truth
    /// for what we feed into the compiler each pass. Open-doc edits and
    /// project loads both write here.
    pub sources: HashMap<String, String>,
    /// Per-URL editor state (line index, version).
    pub docs: OpenDocs,
    /// LSP-side line indices for project files we've loaded from disk
    /// but the editor hasn't opened. Keyed by compiler path. Used so we
    /// can publish diagnostics in non-open files (e.g. when an open file
    /// triggers an error in a closed dep).
    pub disk_line_indices: HashMap<String, LineIndex>,
    /// Workspace roots received from `initialize`.
    pub workspace_roots: Vec<PathBuf>,
    /// URLs we've published diagnostics for at least once. We send empty
    /// diagnostics on `didClose` so stale squiggles disappear.
    pub published: HashSet<Url>,
    /// Bumped on every edit. Stale analysis tasks compare against this and
    /// drop their results if the world has moved on.
    pub revision_token: u64,
}

impl Default for ServerState {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerState {
    pub fn new() -> Self {
        Self {
            sources: HashMap::new(),
            docs: OpenDocs::default(),
            disk_line_indices: HashMap::new(),
            workspace_roots: Vec::new(),
            published: HashSet::new(),
            revision_token: 0,
        }
    }

    /// Set the source for a path, overwriting any previous text. Returns
    /// `true` if the value actually changed.
    pub fn set_source(&mut self, path: String, text: String) -> bool {
        match self.sources.get(&path) {
            Some(existing) if existing == &text => false,
            _ => {
                self.sources.insert(path, text);
                true
            },
        }
    }
}

pub type SharedState = Arc<Mutex<ServerState>>;

/// Convert a Kestrel file path (compiler key) to an LSP `Url`.
pub fn path_to_url(path: &str) -> Option<Url> {
    let pb = PathBuf::from(path);
    if pb.is_absolute() {
        Url::from_file_path(&pb).ok()
    } else {
        None
    }
}

/// Convert an LSP `Url` to a compiler-friendly path string. Canonicalizes
/// when the file exists so two URLs that point at the same on-disk file
/// land on the same compiler entity.
pub fn url_to_path(uri: &Url) -> String {
    if let Ok(pb) = uri.to_file_path() {
        if let Ok(canon) = pb.canonicalize() {
            return canon.to_string_lossy().into_owned();
        }
        return pb.to_string_lossy().into_owned();
    }
    uri.to_string()
}

/// Build a fresh `Compiler` from the current `sources` map and run the AST
/// builder on every loaded file. Returns the compiler plus a path → entity
/// map that handlers use for span resolution.
///
/// `Compiler::new()` re-seeds the lang module each call. Profile shows this
/// is dominated by inference, not setup, on realistic projects — but if it
/// becomes a hot spot we can switch to a snapshot-clone pattern.
pub fn rebuild_compiler(sources: &HashMap<String, String>) -> (Compiler, HashMap<String, Entity>) {
    let mut compiler = Compiler::new();
    let mut paths_in_order: Vec<&String> = sources.keys().collect();
    paths_in_order.sort();

    let mut by_path = HashMap::new();
    for path in paths_in_order {
        let text = &sources[path];
        let entity = compiler.set_source(path, text.clone());
        compiler.build(entity);
        by_path.insert(path.clone(), entity);
    }
    (compiler, by_path)
}
