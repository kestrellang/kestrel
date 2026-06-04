//! Server state shared between request handlers.
//!
//! Strategy: a single canonical `sources: HashMap<path, text>` map plus
//! a `CompilerHandle` to a worker thread that owns the persistent
//! `Compiler`. Every handler request and every diagnostics refresh
//! sends a job through the handle; the worker syncs its `Compiler` to
//! the requested source state (Phase 1: stdlib loaded once, user code
//! rebuilt as a unit on any user-side change) and runs the closure.
//! See [`crate::compiler_worker`] for the invalidation policy.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::Mutex;
use tower_lsp::lsp_types::Url;

use crate::compiler_worker::CompilerHandle;
use crate::documents::OpenDocs;
use crate::position::LineIndex;

pub struct ServerState {
    /// Compiler-key (canonical path) → source text. Single source of truth
    /// for what we feed into the compiler each pass. Open-doc edits and
    /// project loads both write here.
    pub sources: HashMap<String, String>,
    /// Subset of `sources` that came from `kestrel.stdlibPath`. The
    /// worker treats these as immutable and never despawns their
    /// entities (see [`crate::compiler_worker`]). Workspace files are
    /// "user code" — anything in `sources` not in this set.
    pub stdlib_paths: HashSet<String>,
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
    /// `kestrel.stdlibPath` from `initializationOptions`. When set, the
    /// project loader walks this directory for `.ks` sources and includes
    /// them in every build. Empty/None = compiler-default behaviour.
    pub stdlib_path: Option<PathBuf>,
    /// `kestrel.flockCachePath` from `initializationOptions`. Overrides
    /// flock's `~/.kestrel/packages` when resolving registry deps.
    pub flock_cache_path: Option<PathBuf>,
    /// Handle to the worker thread that owns the persistent `Compiler`.
    /// Cloneable; handler tasks clone this to send their own jobs.
    pub compiler_handle: CompilerHandle,
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
            stdlib_paths: HashSet::new(),
            docs: OpenDocs::default(),
            disk_line_indices: HashMap::new(),
            workspace_roots: Vec::new(),
            published: HashSet::new(),
            revision_token: 0,
            stdlib_path: None,
            flock_cache_path: None,
            compiler_handle: CompilerHandle::spawn(),
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

    /// Snapshot the current source set into stdlib + user maps for
    /// shipping to the compiler worker. Both maps are cheap to clone
    /// out of the state lock — they're shared via `Arc` from there on.
    pub fn partition_sources(
        &self,
    ) -> (Arc<HashMap<String, String>>, Arc<HashMap<String, String>>) {
        let mut stdlib = HashMap::new();
        let mut user = HashMap::new();
        for (path, text) in &self.sources {
            if self.stdlib_paths.contains(path) {
                stdlib.insert(path.clone(), text.clone());
            } else {
                user.insert(path.clone(), text.clone());
            }
        }
        (Arc::new(stdlib), Arc::new(user))
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
