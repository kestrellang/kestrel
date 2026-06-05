//! Kestrel language server.
//!
//! Speaks LSP over stdio (see `main.rs`). The `Backend` implements
//! `tower_lsp::LanguageServer`; per-feature logic lives in `handlers/`.
//!
//! See `lib/kestrel-lsp/CHECKLIST.md` for milestone progress.

pub mod compiler_worker;
pub mod convert;
pub mod documents;
pub mod handlers;
pub mod position;
pub mod project;
pub mod references;
pub mod semantic;
pub mod server;
pub mod syntax;
pub mod ty_format;
pub mod types;

use std::sync::Arc;

use tokio::sync::{Mutex, Notify};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::server::{ServerState, SharedState};

/// Debounce delay for `didChange` → diagnostics refresh. Coalesces rapid
/// keystrokes so we don't rebuild the world on every character.
const DEBOUNCE_MS: u64 = 150;

pub struct Backend {
    client: Client,
    state: SharedState,
    /// Poked by `did_change`; the debounce task waits on this and
    /// coalesces notifications before triggering a refresh.
    debounce_notify: Arc<Notify>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        let state: SharedState = Arc::new(Mutex::new(ServerState::new()));
        let debounce_notify = Arc::new(Notify::new());

        // Spawn the debounce task. It waits for a notification, then
        // sleeps DEBOUNCE_MS — resetting on each new notification — and
        // fires a refresh only after the dust settles.
        {
            let state = state.clone();
            let client = client.clone();
            let notify = debounce_notify.clone();
            tokio::spawn(async move {
                loop {
                    notify.notified().await;
                    loop {
                        tokio::select! {
                            _ = tokio::time::sleep(
                                std::time::Duration::from_millis(DEBOUNCE_MS),
                            ) => break,
                            _ = notify.notified() => continue,
                        }
                    }
                    handlers::diagnostics::refresh(state.clone(), client.clone()).await;
                }
            });
        }

        Self {
            client,
            state,
            debounce_notify,
        }
    }

    async fn refresh(&self) {
        handlers::diagnostics::refresh(self.state.clone(), self.client.clone()).await;
    }

    /// Walk every workspace folder, find `flock.toml` manifests, and load all
    /// `.ks` sources from those packages (and their path deps) into the
    /// source map. Disk-loaded files get a `LineIndex` so we can publish
    /// diagnostics in them even when they're not open in the editor.
    async fn load_workspace(&self, folders: Vec<WorkspaceFolder>) {
        // Snapshot settings before scanning so the heavy filesystem walk
        // doesn't hold the state mutex.
        let (stdlib_path, cache_path) = {
            let s = self.state.lock().await;
            (s.stdlib_path.clone(), s.flock_cache_path.clone())
        };

        let mut new_sources: Vec<(String, String)> = Vec::new();
        let mut new_stdlib_paths: Vec<String> = Vec::new();
        let mut cache_misses: Vec<String> = Vec::new();

        // Stdlib first so workspace files (which may import std modules)
        // can resolve. Loaded only if `kestrel.stdlibPath` is configured.
        // Stdlib paths are tracked separately so the compiler worker can
        // partition stdlib (immutable, cached forever) from user code
        // (rebuilt on edit).
        if let Some(stdlib) = stdlib_path {
            let mut paths: Vec<std::path::PathBuf> = Vec::new();
            project::walk_kestrel_sources(&stdlib, &mut paths);
            for path in paths {
                let Ok(canon) = path.canonicalize() else {
                    continue;
                };
                let key = canon.to_string_lossy().into_owned();
                let Ok(text) = std::fs::read_to_string(&canon) else {
                    continue;
                };
                new_stdlib_paths.push(key.clone());
                new_sources.push((key, text));
            }
        }

        for folder in &folders {
            let Ok(root) = folder.uri.to_file_path() else {
                continue;
            };
            // Find every manifest at any depth — workspace folders sometimes
            // contain multiple packages side-by-side (e.g. `examples/`).
            for entry in walkdir::WalkDir::new(&root)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_name() == "flock.toml" && entry.file_type().is_file() {
                    let report = project::collect_sources(entry.path(), cache_path.as_deref());
                    for path in report.sources {
                        let Ok(canon) = path.canonicalize() else {
                            continue;
                        };
                        let key = canon.to_string_lossy().into_owned();
                        let Ok(text) = std::fs::read_to_string(&canon) else {
                            continue;
                        };
                        new_sources.push((key, text));
                    }
                    cache_misses.extend(report.missing_cache);
                }
            }
        }

        let mut state = self.state.lock().await;
        for folder in folders {
            if let Ok(p) = folder.uri.to_file_path() {
                state.workspace_roots.push(p);
            }
        }
        for path in new_stdlib_paths {
            state.stdlib_paths.insert(path);
        }
        for (path, text) in new_sources {
            state
                .disk_line_indices
                .insert(path.clone(), position::LineIndex::new(text.clone()));
            state.sources.insert(path, text);
        }
        state.revision_token += 1;
        drop(state);

        // Surface registry cache misses to the editor's output channel so
        // the user knows to run `flock build`.
        for missing in cache_misses {
            self.client
                .log_message(
                    MessageType::WARNING,
                    format!(
                        "Kestrel: registry dep {missing} not in flock cache. Run `flock build` to fetch."
                    ),
                )
                .await;
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        // Read initializationOptions before loading sources so the project
        // walker can honor stdlibPath / flockCachePath on the first pass.
        if let Some(opts) = params.initialization_options.as_ref() {
            let stdlib = opts
                .get("stdlibPath")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(std::path::PathBuf::from);
            let cache = opts
                .get("flockCachePath")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(std::path::PathBuf::from);
            let mut state = self.state.lock().await;
            // Auto-discover stdlib if not explicitly configured:
            //   1. KESTREL_STD env var
            //   2. <exe>/../lib/std (jessup toolchain layout)
            state.stdlib_path = stdlib.or_else(default_std_path);
            state.flock_cache_path = cache;
        } else {
            let mut state = self.state.lock().await;
            state.stdlib_path = default_std_path();
        }

        if let Some(folders) = params.workspace_folders {
            self.load_workspace(folders).await;
        } else if let Some(root_uri) = params.root_uri {
            self.load_workspace(vec![WorkspaceFolder {
                uri: root_uri,
                name: String::new(),
            }])
            .await;
        }

        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "kestrel-lsp".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".into()]),
                    ..Default::default()
                }),
                references_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: Default::default(),
                })),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                code_lens_provider: Some(CodeLensOptions {
                    resolve_provider: Some(false),
                }),
                signature_help_provider: Some(SignatureHelpOptions {
                    trigger_characters: Some(vec!["(".into(), ",".into()]),
                    retrigger_characters: None,
                    work_done_progress_options: Default::default(),
                }),
                document_highlight_provider: Some(OneOf::Left(true)),
                workspace_symbol_provider: Some(OneOf::Left(true)),
                call_hierarchy_provider: Some(CallHierarchyServerCapability::Simple(true)),
                inlay_hint_provider: Some(OneOf::Left(true)),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: SemanticTokensLegend {
                                token_types: handlers::semantic_tokens::LEGEND.to_vec(),
                                token_modifiers: vec![],
                            },
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            range: None,
                            ..Default::default()
                        },
                    ),
                ),
                ..Default::default()
            },
        })
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        Ok(handlers::hover::handle(self.state.clone(), params).await)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        Ok(handlers::definition::handle(self.state.clone(), params).await)
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        Ok(handlers::semantic_tokens::handle(self.state.clone(), params).await)
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        Ok(handlers::completion::handle(self.state.clone(), params).await)
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        Ok(handlers::references::handle(self.state.clone(), params).await)
    }

    async fn document_highlight(
        &self,
        params: DocumentHighlightParams,
    ) -> Result<Option<Vec<DocumentHighlight>>> {
        Ok(handlers::document_highlight::handle(self.state.clone(), params).await)
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        Ok(handlers::document_symbols::handle(self.state.clone(), params).await)
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        handlers::rename::prepare(self.state.clone(), params).await
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        handlers::rename::rename(self.state.clone(), params).await
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        Ok(handlers::code_actions::handle(self.state.clone(), params).await)
    }

    async fn code_lens(&self, params: CodeLensParams) -> Result<Option<Vec<CodeLens>>> {
        Ok(handlers::code_lens::handle(self.state.clone(), params).await)
    }

    async fn signature_help(&self, params: SignatureHelpParams) -> Result<Option<SignatureHelp>> {
        Ok(handlers::signature_help::handle(self.state.clone(), params).await)
    }

    async fn inlay_hint(&self, params: InlayHintParams) -> Result<Option<Vec<InlayHint>>> {
        Ok(handlers::inlay_hints::handle(self.state.clone(), params).await)
    }

    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> Result<Option<Vec<SymbolInformation>>> {
        Ok(handlers::workspace_symbols::handle(self.state.clone(), params).await)
    }

    async fn prepare_call_hierarchy(
        &self,
        params: CallHierarchyPrepareParams,
    ) -> Result<Option<Vec<CallHierarchyItem>>> {
        Ok(handlers::call_hierarchy::prepare(self.state.clone(), params).await)
    }

    async fn incoming_calls(
        &self,
        params: CallHierarchyIncomingCallsParams,
    ) -> Result<Option<Vec<CallHierarchyIncomingCall>>> {
        Ok(handlers::call_hierarchy::incoming(self.state.clone(), params).await)
    }

    async fn outgoing_calls(
        &self,
        params: CallHierarchyOutgoingCallsParams,
    ) -> Result<Option<Vec<CallHierarchyOutgoingCall>>> {
        Ok(handlers::call_hierarchy::outgoing(self.state.clone(), params).await)
    }

    async fn prepare_type_hierarchy(
        &self,
        params: TypeHierarchyPrepareParams,
    ) -> Result<Option<Vec<TypeHierarchyItem>>> {
        Ok(handlers::type_hierarchy::prepare(self.state.clone(), params).await)
    }

    async fn supertypes(
        &self,
        params: TypeHierarchySupertypesParams,
    ) -> Result<Option<Vec<TypeHierarchyItem>>> {
        Ok(handlers::type_hierarchy::supertypes(self.state.clone(), params).await)
    }

    async fn subtypes(
        &self,
        params: TypeHierarchySubtypesParams,
    ) -> Result<Option<Vec<TypeHierarchyItem>>> {
        Ok(handlers::type_hierarchy::subtypes(self.state.clone(), params).await)
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "kestrel-lsp ready")
            .await;
        self.refresh().await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        let version = params.text_document.version;
        let key = server::url_to_path(&uri);
        {
            let mut state = self.state.lock().await;
            state.docs.open(uri, text.clone(), version);
            state.set_source(key, text);
            state.revision_token += 1;
        }
        self.refresh().await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;
        // We advertise FULL sync, so there is exactly one content change with
        // the entire new buffer.
        let Some(change) = params.content_changes.into_iter().next() else {
            return;
        };
        let text = change.text;
        let key = server::url_to_path(&uri);
        {
            let mut state = self.state.lock().await;
            state.docs.replace(&uri, text.clone(), version);
            state.set_source(key, text);
            state.revision_token += 1;
        }
        self.debounce_notify.notify_one();
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        {
            let mut state = self.state.lock().await;
            state.docs.close(&uri);
            // Leave the source in `sources` so other open files can still
            // resolve it. We only stop tracking the LSP-side line index.
            state.revision_token += 1;
        }
        // Clear stale diagnostics in the closed file.
        self.client
            .publish_diagnostics(uri.clone(), vec![], None)
            .await;
        {
            let mut state = self.state.lock().await;
            state.published.remove(&uri);
        }
        self.refresh().await;
    }

    async fn did_save(&self, _: DidSaveTextDocumentParams) {
        self.refresh().await;
    }

    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        // The extension watches flock.toml / flock.lock / *.ks. A change to
        // any manifest invalidates the dep set, so we reload the entire
        // workspace. For plain `.ks` edits we just re-read the affected file
        // (the open-doc events already handle live edits — this catches
        // out-of-band changes like git checkout / formatter run).
        let mut needs_workspace_reload = false;
        let mut to_reload: Vec<std::path::PathBuf> = Vec::new();
        let mut to_drop: Vec<String> = Vec::new();

        for event in &params.changes {
            let Ok(path) = event.uri.to_file_path() else {
                continue;
            };
            let name = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            if name == "flock.toml" || name == "flock.lock" {
                needs_workspace_reload = true;
                continue;
            }
            if path.extension().and_then(|s| s.to_str()) != Some("ks") {
                continue;
            }
            match event.typ {
                FileChangeType::DELETED => {
                    if let Ok(canon) = path.canonicalize() {
                        to_drop.push(canon.to_string_lossy().into_owned());
                    } else {
                        to_drop.push(path.to_string_lossy().into_owned());
                    }
                },
                FileChangeType::CREATED | FileChangeType::CHANGED => {
                    to_reload.push(path);
                },
                _ => {},
            }
        }

        if needs_workspace_reload {
            // Snapshot existing roots so we can re-walk them. load_workspace
            // appends, so first clear the path-derived state.
            let folders: Vec<WorkspaceFolder> = {
                let mut state = self.state.lock().await;
                state.sources.clear();
                state.stdlib_paths.clear();
                state.disk_line_indices.clear();
                let folders = state
                    .workspace_roots
                    .iter()
                    .filter_map(|p| Url::from_file_path(p).ok())
                    .map(|uri| WorkspaceFolder {
                        uri,
                        name: String::new(),
                    })
                    .collect();
                state.workspace_roots.clear();
                folders
            };
            self.load_workspace(folders).await;
        } else {
            let mut state = self.state.lock().await;
            for path in to_reload {
                let Ok(canon) = path.canonicalize() else {
                    continue;
                };
                let key = canon.to_string_lossy().into_owned();
                let Ok(text) = std::fs::read_to_string(&canon) else {
                    continue;
                };
                state
                    .disk_line_indices
                    .insert(key.clone(), position::LineIndex::new(text.clone()));
                state.sources.insert(key, text);
            }
            for key in to_drop {
                state.sources.remove(&key);
                state.disk_line_indices.remove(&key);
            }
            state.revision_token += 1;
        }

        self.refresh().await;
    }
}

/// Auto-discover the stdlib directory, in priority order:
///   1. `KESTREL_STD` env var
///   2. `<exe>/../lib/std` (jessup-installed toolchain layout)
///   3. `~/.jessup/bin/kestrel` symlink → resolve to toolchain's lib/std
fn default_std_path() -> Option<std::path::PathBuf> {
    if let Some(p) = std::env::var_os("KESTREL_STD") {
        let p = std::path::PathBuf::from(p);
        if p.exists() {
            return Some(p);
        }
    }

    if let Ok(exe) = std::env::current_exe()
        && let Some(p) = exe
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("lib/std"))
        && p.exists()
    {
        return Some(p);
    }

    // Follow the jessup kestrel symlink to find the active toolchain's stdlib.
    // Covers the case where a bundled VSIX LSP binary can't use exe-relative.
    if let Some(home) = std::env::var_os("HOME") {
        let kestrel_link = std::path::PathBuf::from(home).join(".jessup/bin/kestrel");
        if let Ok(resolved) = std::fs::read_link(&kestrel_link) {
            let std_path = resolved
                .parent()
                .and_then(|p| p.parent())
                .map(|p| p.join("lib/std"));
            if let Some(p) = std_path
                && p.exists()
            {
                return Some(p);
            }
        }
    }

    None
}
