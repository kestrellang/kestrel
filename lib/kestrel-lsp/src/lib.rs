//! Kestrel language server.
//!
//! Speaks LSP over stdio (see `main.rs`). The `Backend` implements
//! `tower_lsp::LanguageServer`; per-feature logic lives in `handlers/`.
//!
//! See `lib/kestrel-lsp/CHECKLIST.md` for milestone progress.

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
        let mut new_sources: Vec<(String, String)> = Vec::new();
        for folder in &folders {
            let Ok(root) = folder.uri.to_file_path() else { continue };
            // Find every manifest at any depth — workspace folders sometimes
            // contain multiple packages side-by-side (e.g. `examples/`).
            for entry in walkdir::WalkDir::new(&root)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_name() == "flock.toml" && entry.file_type().is_file() {
                    for path in project::collect_sources(entry.path()) {
                        let Ok(canon) = path.canonicalize() else { continue };
                        let key = canon.to_string_lossy().into_owned();
                        let Ok(text) = std::fs::read_to_string(&canon) else {
                            continue;
                        };
                        new_sources.push((key, text));
                    }
                }
            }
        }

        let mut state = self.state.lock().await;
        for folder in folders {
            if let Ok(p) = folder.uri.to_file_path() {
                state.workspace_roots.push(p);
            }
        }
        for (path, text) in new_sources {
            state.disk_line_indices.insert(path.clone(), position::LineIndex::new(text.clone()));
            state.sources.insert(path, text);
        }
        state.revision_token += 1;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
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
}
