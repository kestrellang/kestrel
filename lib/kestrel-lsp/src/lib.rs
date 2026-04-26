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

use std::sync::Arc;

use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::server::{ServerState, SharedState};

pub struct Backend {
    client: Client,
    state: SharedState,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            state: Arc::new(Mutex::new(ServerState::new())),
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
                let Ok(canon) = path.canonicalize() else { continue };
                let key = canon.to_string_lossy().into_owned();
                let Ok(text) = std::fs::read_to_string(&canon) else { continue };
                new_stdlib_paths.push(key.clone());
                new_sources.push((key, text));
            }
        }

        for folder in &folders {
            let Ok(root) = folder.uri.to_file_path() else { continue };
            // Find every manifest at any depth — workspace folders sometimes
            // contain multiple packages side-by-side (e.g. `examples/`).
            for entry in walkdir::WalkDir::new(&root)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_name() == "flock.toml" && entry.file_type().is_file() {
                    let report =
                        project::collect_sources(entry.path(), cache_path.as_deref());
                    for path in report.sources {
                        let Ok(canon) = path.canonicalize() else { continue };
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
            state.disk_line_indices.insert(path.clone(), position::LineIndex::new(text.clone()));
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
            state.stdlib_path = stdlib;
            state.flock_cache_path = cache;
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
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
                        legend: SemanticTokensLegend {
                            token_types: handlers::semantic_tokens::LEGEND.to_vec(),
                            token_modifiers: vec![],
                        },
                        full: Some(SemanticTokensFullOptions::Bool(true)),
                        range: None,
                        ..Default::default()
                    }),
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

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> Result<Option<CompletionResponse>> {
        Ok(handlers::completion::handle(self.state.clone(), params).await)
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        Ok(handlers::references::handle(self.state.clone(), params).await)
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

    async fn code_action(
        &self,
        params: CodeActionParams,
    ) -> Result<Option<CodeActionResponse>> {
        Ok(handlers::code_actions::handle(self.state.clone(), params).await)
    }

    async fn code_lens(&self, params: CodeLensParams) -> Result<Option<Vec<CodeLens>>> {
        Ok(handlers::code_lens::handle(self.state.clone(), params).await)
    }

    async fn signature_help(
        &self,
        params: SignatureHelpParams,
    ) -> Result<Option<SignatureHelp>> {
        Ok(handlers::signature_help::handle(self.state.clone(), params).await)
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
        let Some(change) = params.content_changes.into_iter().next() else { return };
        let text = change.text;
        let key = server::url_to_path(&uri);
        {
            let mut state = self.state.lock().await;
            state.docs.replace(&uri, text.clone(), version);
            state.set_source(key, text);
            state.revision_token += 1;
        }
        self.refresh().await;
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
                let Ok(canon) = path.canonicalize() else { continue };
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
