//! LSP Backend implementation.
//!
//! Implements the Language Server Protocol for Kestrel using tower-lsp.

use crate::diagnostics::{convert_diagnostics_for_file, position_to_byte_offset, byte_offset_to_position};
use crate::position::{find_symbol_at_position, find_call_site_at_position, find_dot_completions, CompletionKind};
use dashmap::DashMap;
use kestrel_compiler::Compilation;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

/// Result of compilation: diagnostics per file.
type CompilationResult = Vec<(Url, Vec<Diagnostic>)>;

/// The LSP backend for Kestrel.
pub struct Backend {
    /// The LSP client for sending notifications.
    client: Client,
    /// Open documents keyed by URI.
    documents: DashMap<Url, String>,
    /// Workspace root path.
    workspace_root: RwLock<Option<PathBuf>>,
}

impl Backend {
    /// Create a new backend instance.
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: DashMap::new(),
            workspace_root: RwLock::new(None),
        }
    }

    /// Compile all open documents and publish diagnostics.
    async fn compile_and_publish(&self) {
        // Collect all documents
        let documents: Vec<(Url, String)> = self
            .documents
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();

        if documents.is_empty() {
            return;
        }

        // Run compilation in a blocking task
        // We process diagnostics inside the task since Compilation isn't Send
        let compilation_result: Option<CompilationResult> =
            tokio::task::spawn_blocking(move || {
                // Build sources for compilation
                // Use the URI path as the filename
                let sources: Vec<(String, String)> = documents
                    .iter()
                    .map(|(uri, content)| {
                        let name = uri
                            .path_segments()
                            .and_then(|mut s| s.next_back())
                            .unwrap_or("unknown.ks")
                            .to_string();
                        (name, content.clone())
                    })
                    .collect();

                // Create compilation
                let mut builder = Compilation::builder();
                for (name, source) in &sources {
                    builder = builder.add_source(name.clone(), source.clone());
                }

                match builder.build() {
                    Ok(compilation) => {
                        // Build a map from filename to file_id (index in source_files)
                        // This accounts for stdlib files which are added first
                        let file_id_map: HashMap<&str, usize> = compilation
                            .source_files()
                            .iter()
                            .enumerate()
                            .map(|(id, sf)| (sf.name(), id))
                            .collect();

                        let all_diagnostics = compilation.diagnostics().diagnostics();

                        let results: CompilationResult = documents
                            .iter()
                            .enumerate()
                            .map(|(i, (uri, _))| {
                                let filename = &sources[i].0;
                                let source = &sources[i].1;

                                // Look up the actual file_id for this filename
                                let file_id = file_id_map.get(filename.as_str()).copied();

                                let lsp_diagnostics = if let Some(file_id) = file_id {
                                    convert_diagnostics_for_file(all_diagnostics, file_id, source)
                                } else {
                                    vec![]
                                };

                                (uri.clone(), lsp_diagnostics)
                            })
                            .collect();

                        Some(results)
                    }
                    Err(e) => {
                        eprintln!("Stdlib error: {}", e);
                        None
                    }
                }
            })
            .await
            .ok()
            .flatten();

        // Publish diagnostics
        match compilation_result {
            Some(results) => {
                for (uri, diagnostics) in results {
                    self.client
                        .publish_diagnostics(uri, diagnostics, None)
                        .await;
                }
            }
            None => {
                // Clear diagnostics on error
                for entry in self.documents.iter() {
                    self.client
                        .publish_diagnostics(entry.key().clone(), vec![], None)
                        .await;
                }
            }
        }
    }

}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        // Store workspace root if provided
        if let Some(root_uri) = params.root_uri
            && let Ok(path) = root_uri.to_file_path()
        {
            *self.workspace_root.write().await = Some(path);
        }

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                signature_help_provider: Some(SignatureHelpOptions {
                    trigger_characters: Some(vec!["(".to_string(), ",".to_string()]),
                    retrigger_characters: None,
                    work_done_progress_options: Default::default(),
                }),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".to_string()]),
                    resolve_provider: Some(false),
                    work_done_progress_options: Default::default(),
                    all_commit_characters: None,
                    completion_item: None,
                }),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "kestrel-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Kestrel LSP server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;

        self.documents.insert(uri, text);
        self.compile_and_publish().await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;

        // With full sync, we get the entire document content
        if let Some(change) = params.content_changes.into_iter().last() {
            self.documents.insert(uri, change.text);
            self.compile_and_publish().await;
        }
    }

    async fn did_save(&self, _params: DidSaveTextDocumentParams) {
        // Re-compile on save (in case external changes affect compilation)
        self.compile_and_publish().await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;

        // Remove document and clear its diagnostics
        self.documents.remove(&uri);
        self.client.publish_diagnostics(uri, vec![], None).await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        // Run in blocking task since compilation isn't async-safe
        let uri_clone = uri.clone();
        let documents = self.documents.clone();

        let result = tokio::task::spawn_blocking(move || {
            // Get document content
            let content = documents.get(&uri_clone)?.value().clone();
            let filename = uri_clone
                .path_segments()
                .and_then(|mut s| s.next_back())
                .unwrap_or("unknown.ks")
                .to_string();

            // Collect all documents for compilation
            let all_docs: Vec<(String, String)> = documents
                .iter()
                .map(|entry| {
                    let name = entry
                        .key()
                        .path_segments()
                        .and_then(|mut s| s.next_back())
                        .unwrap_or("unknown.ks")
                        .to_string();
                    (name, entry.value().clone())
                })
                .collect();

            // Convert position to byte offset
            let offset = position_to_byte_offset(&content, position);

            // Build compilation
            let mut builder = Compilation::builder();
            for (name, source) in &all_docs {
                builder = builder.add_source(name.clone(), source.clone());
            }

            let compilation = builder.build().ok()?;

            // Find file_id for this file
            let file_id = compilation
                .source_files()
                .iter()
                .enumerate()
                .find(|(_, sf)| sf.name() == filename)
                .map(|(id, _)| id)?;

            // Find symbol at position
            let symbol_info = find_symbol_at_position(&compilation, file_id, offset)?;

            Some(symbol_info)
        })
        .await
        .ok()
        .flatten();

        Ok(result.map(|info| Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!("```kestrel\n{}\n```", info.signature),
            }),
            range: None,
        }))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        // Run in blocking task since compilation isn't async-safe
        let uri_clone = uri.clone();
        let documents = self.documents.clone();

        let result = tokio::task::spawn_blocking(move || {
            // Get document content
            let content = documents.get(&uri_clone)?.value().clone();
            let filename = uri_clone
                .path_segments()
                .and_then(|mut s| s.next_back())
                .unwrap_or("unknown.ks")
                .to_string();

            // Collect all documents for compilation
            let all_docs: Vec<(String, String)> = documents
                .iter()
                .map(|entry| {
                    let name = entry
                        .key()
                        .path_segments()
                        .and_then(|mut s| s.next_back())
                        .unwrap_or("unknown.ks")
                        .to_string();
                    (name, entry.value().clone())
                })
                .collect();

            // Convert position to byte offset
            let offset = position_to_byte_offset(&content, position);

            // Build compilation
            let mut builder = Compilation::builder();
            for (name, source) in &all_docs {
                builder = builder.add_source(name.clone(), source.clone());
            }

            let compilation = builder.build().ok()?;

            // Build file_id to source map
            let mut file_id_to_source: HashMap<usize, String> = HashMap::new();
            let mut file_id_to_uri: HashMap<usize, Url> = HashMap::new();

            for (id, sf) in compilation.source_files().iter().enumerate() {
                file_id_to_source.insert(id, sf.source().to_string());

                // Try to find matching URI for this file
                for entry in documents.iter() {
                    let entry_name = entry
                        .key()
                        .path_segments()
                        .and_then(|mut s| s.next_back())
                        .unwrap_or("");
                    if entry_name == sf.name() {
                        file_id_to_uri.insert(id, entry.key().clone());
                        break;
                    }
                }
            }

            // Find file_id for this file
            let file_id = compilation
                .source_files()
                .iter()
                .enumerate()
                .find(|(_, sf)| sf.name() == filename)
                .map(|(id, _)| id)?;

            // Find symbol at position
            let symbol_info = find_symbol_at_position(&compilation, file_id, offset)?;

            // Get definition location
            let (def_file_id, def_start, def_end) = symbol_info.definition?;

            // Get source for the definition file
            let def_source = file_id_to_source.get(&def_file_id)?;

            // Convert byte offsets to positions
            let start_pos = byte_offset_to_position(def_source, def_start);
            let end_pos = byte_offset_to_position(def_source, def_end);

            // Get URI for definition file (use current file if same, or try to find it)
            let def_uri = file_id_to_uri.get(&def_file_id).cloned().unwrap_or(uri_clone);

            Some(GotoDefinitionResponse::Scalar(Location {
                uri: def_uri,
                range: Range::new(start_pos, end_pos),
            }))
        })
        .await
        .ok()
        .flatten();

        Ok(result)
    }

    async fn signature_help(&self, params: SignatureHelpParams) -> Result<Option<SignatureHelp>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        // Run in blocking task since compilation isn't async-safe
        let uri_clone = uri.clone();
        let documents = self.documents.clone();

        let result = tokio::task::spawn_blocking(move || {
            // Get document content
            let content = documents.get(&uri_clone)?.value().clone();
            let filename = uri_clone
                .path_segments()
                .and_then(|mut s| s.next_back())
                .unwrap_or("unknown.ks")
                .to_string();

            // Collect all documents for compilation
            let all_docs: Vec<(String, String)> = documents
                .iter()
                .map(|entry| {
                    let name = entry
                        .key()
                        .path_segments()
                        .and_then(|mut s| s.next_back())
                        .unwrap_or("unknown.ks")
                        .to_string();
                    (name, entry.value().clone())
                })
                .collect();

            // Convert position to byte offset
            let offset = position_to_byte_offset(&content, position);

            // Build compilation
            let mut builder = Compilation::builder();
            for (name, source) in &all_docs {
                builder = builder.add_source(name.clone(), source.clone());
            }

            let compilation = builder.build().ok()?;

            // Find file_id for this file
            let file_id = compilation
                .source_files()
                .iter()
                .enumerate()
                .find(|(_, sf)| sf.name() == filename)
                .map(|(id, _)| id)?;

            // Find call site at position
            let call_info = find_call_site_at_position(&compilation, &content, file_id, offset)?;

            // Build signature information
            let parameters: Vec<ParameterInformation> = call_info
                .parameters
                .iter()
                .map(|p| ParameterInformation {
                    label: ParameterLabel::Simple(p.label.clone()),
                    documentation: p.documentation.clone().map(|d| {
                        Documentation::MarkupContent(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: d,
                        })
                    }),
                })
                .collect();

            let signature_label = format!(
                "func {}({}) -> {}",
                call_info.function_name,
                call_info
                    .parameters
                    .iter()
                    .map(|p| p.label.clone())
                    .collect::<Vec<_>>()
                    .join(", "),
                call_info.return_type
            );

            Some(SignatureHelp {
                signatures: vec![SignatureInformation {
                    label: signature_label,
                    documentation: None,
                    parameters: Some(parameters),
                    active_parameter: Some(call_info.active_parameter as u32),
                }],
                active_signature: Some(0),
                active_parameter: Some(call_info.active_parameter as u32),
            })
        })
        .await
        .ok()
        .flatten();

        Ok(result)
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        // Run in blocking task since compilation isn't async-safe
        let uri_clone = uri.clone();
        let documents = self.documents.clone();

        let result = tokio::task::spawn_blocking(move || {
            // Get document content
            let content = documents.get(&uri_clone)?.value().clone();
            let filename = uri_clone
                .path_segments()
                .and_then(|mut s| s.next_back())
                .unwrap_or("unknown.ks")
                .to_string();

            // Collect all documents for compilation
            let all_docs: Vec<(String, String)> = documents
                .iter()
                .map(|entry| {
                    let name = entry
                        .key()
                        .path_segments()
                        .and_then(|mut s| s.next_back())
                        .unwrap_or("unknown.ks")
                        .to_string();
                    (name, entry.value().clone())
                })
                .collect();

            // Convert position to byte offset
            let offset = position_to_byte_offset(&content, position);

            // Build compilation
            let mut builder = Compilation::builder();
            for (name, source) in &all_docs {
                builder = builder.add_source(name.clone(), source.clone());
            }

            let compilation = builder.build().ok()?;

            // Find file_id for this file
            let file_id = compilation
                .source_files()
                .iter()
                .enumerate()
                .find(|(_, sf)| sf.name() == filename)
                .map(|(id, _)| id)?;

            // Find completions
            let completions = find_dot_completions(&compilation, &content, file_id, offset);

            if completions.is_empty() {
                return None;
            }

            // Convert to LSP completion items
            let items: Vec<tower_lsp::lsp_types::CompletionItem> = completions
                .into_iter()
                .map(|c| {
                    let kind = match c.kind {
                        CompletionKind::Field => CompletionItemKind::FIELD,
                        CompletionKind::Method => CompletionItemKind::METHOD,
                        CompletionKind::Function => CompletionItemKind::FUNCTION,
                        CompletionKind::Property => CompletionItemKind::PROPERTY,
                    };
                    tower_lsp::lsp_types::CompletionItem {
                        label: c.label,
                        kind: Some(kind),
                        detail: c.detail,
                        insert_text: c.insert_text,
                        ..Default::default()
                    }
                })
                .collect();

            Some(CompletionResponse::Array(items))
        })
        .await
        .ok()
        .flatten();

        Ok(result)
    }
}
