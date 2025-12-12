use std::collections::HashMap;
use std::sync::Arc;

use kestrel_reporting::DiagnosticContext;
use kestrel_semantic_model::SemanticModel;
use kestrel_semantic_tree::behavior::visibility::{Visibility, VisibilityBehavior};
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::module::ModuleSymbol;
use kestrel_semantic_tree::symbol::source_file::SourceFileSymbol;
use kestrel_span::{Span, Spanned};
use kestrel_syntax_tree::SyntaxNode;
use semantic_tree::symbol::{Symbol, SymbolId, SymbolMetadata, SymbolMetadataBuilder};

/// A single input file for semantic lowering.
pub struct BuildFile<'a> {
    pub file_name: &'a str,
    pub syntax: &'a SyntaxNode,
    pub source: &'a str,
    pub file_id: usize,
}

/// Builds a `SemanticModel` from syntax trees (build/lowering phase only).
pub struct SemanticModelBuilder {
    root: Arc<dyn Symbol<KestrelLanguage>>,
    syntax_map: HashMap<SymbolId, SyntaxNode>,
    sources: HashMap<String, String>,
}

impl SemanticModelBuilder {
    pub fn new() -> Self {
        Self {
            root: Arc::new(RootSymbol::new()),
            syntax_map: HashMap::new(),
            sources: HashMap::new(),
        }
    }

    pub fn add_file(
        &mut self,
        file_name: &str,
        syntax: &SyntaxNode,
        source: &str,
        _diagnostics: &mut DiagnosticContext,
        _file_id: usize,
    ) {
        let root = self.root.clone();

        let parent_module = match extract_module_path(syntax) {
            Some(path_segments) if !path_segments.is_empty() => {
                build_module_hierarchy(&root, &path_segments)
            }
            _ => root.clone(),
        };

        let file_name_spanned = Spanned::new(file_name.to_string(), Span::from(0..file_name.len()));
        let source_file_symbol: Arc<dyn Symbol<KestrelLanguage>> = Arc::new(SourceFileSymbol::new(
            file_name_spanned,
            Span::from(0..source.len()),
        ));

        parent_module.metadata().add_child(&source_file_symbol);

        for child in syntax.children() {
            if child.kind() == kestrel_syntax_tree::SyntaxKind::ModuleDeclaration {
                continue;
            }
            self.walk_node(&child, source, Some(&source_file_symbol), &root);
        }

        self.sources
            .insert(file_name.to_string(), source.to_string());
    }

    pub fn build(self) -> SemanticModel {
        SemanticModel::new(self.root, self.syntax_map, self.sources)
    }

    fn walk_node(
        &mut self,
        syntax: &SyntaxNode,
        source: &str,
        parent: Option<&Arc<dyn Symbol<KestrelLanguage>>>,
        root: &Arc<dyn Symbol<KestrelLanguage>>,
    ) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
        if let Some(builder) = builder_for(syntax.kind()) {
            if let Some(symbol) = builder.build_declaration(syntax, source, parent, root) {
                self.syntax_map.insert(symbol.metadata().id(), syntax.clone());

                if !builder.is_terminal() {
                    for child in syntax.children() {
                        self.walk_node(&child, source, Some(&symbol), root);
                    }
                }

                return Some(symbol);
            }
        }

        for child in syntax.children() {
            self.walk_node(&child, source, parent, root);
        }

        None
    }
}

impl Default for SemanticModelBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Lower one or more syntax trees into a `SemanticModel`.
pub fn build<'a, I>(files: I, diagnostics: &mut DiagnosticContext) -> SemanticModel
where
    I: IntoIterator<Item = BuildFile<'a>>,
{
    let mut builder = SemanticModelBuilder::new();

    for file in files {
        builder.add_file(
            file.file_name,
            file.syntax,
            file.source,
            diagnostics,
            file.file_id,
        );
    }

    builder.build()
}

fn builder_for(kind: kestrel_syntax_tree::SyntaxKind) -> Option<&'static dyn crate::builder::Builder>
{
    use crate::builders::{
        ExtensionBuilder, FieldBuilder, FunctionBuilder, ImportBuilder, InitializerBuilder,
        ModuleBuilder, ProtocolBuilder, StructBuilder, TerminalBuilder, TypeAliasBuilder,
    };
    use kestrel_syntax_tree::SyntaxKind;

    static EXTENSION: ExtensionBuilder = ExtensionBuilder;
    static FIELD: FieldBuilder = FieldBuilder;
    static FUNCTION: FunctionBuilder = FunctionBuilder;
    static IMPORT: ImportBuilder = ImportBuilder;
    static INITIALIZER: InitializerBuilder = InitializerBuilder;
    static MODULE: ModuleBuilder = ModuleBuilder;
    static PROTOCOL: ProtocolBuilder = ProtocolBuilder;
    static STRUCT: StructBuilder = StructBuilder;
    static TERMINAL: TerminalBuilder = TerminalBuilder;
    static TYPE_ALIAS: TypeAliasBuilder = TypeAliasBuilder;

    match kind {
        SyntaxKind::ModuleDeclaration => Some(&MODULE),
        SyntaxKind::ImportDeclaration => Some(&IMPORT),
        SyntaxKind::TypeAliasDeclaration => Some(&TYPE_ALIAS),
        SyntaxKind::ProtocolDeclaration => Some(&PROTOCOL),
        SyntaxKind::StructDeclaration => Some(&STRUCT),
        SyntaxKind::ExtensionDeclaration => Some(&EXTENSION),
        SyntaxKind::FieldDeclaration => Some(&FIELD),
        SyntaxKind::FunctionDeclaration => Some(&FUNCTION),
        SyntaxKind::InitializerDeclaration => Some(&INITIALIZER),
        SyntaxKind::Visibility | SyntaxKind::Name => Some(&TERMINAL),
        _ => None,
    }
}

fn extract_module_path(syntax: &SyntaxNode) -> Option<Vec<String>> {
    use kestrel_syntax_tree::SyntaxKind;

    let module_decl = syntax
        .children()
        .find(|child| child.kind() == SyntaxKind::ModuleDeclaration)?;

    let module_path_node = module_decl
        .children()
        .find(|child| child.kind() == SyntaxKind::ModulePath)?;

    Some(
        module_path_node
            .children_with_tokens()
            .filter_map(|elem| elem.into_token())
            .filter(|tok| tok.kind() == SyntaxKind::Identifier)
            .map(|tok| tok.text().to_string())
            .collect(),
    )
}

fn build_module_hierarchy(
    root: &Arc<dyn Symbol<KestrelLanguage>>,
    path_segments: &[String],
) -> Arc<dyn Symbol<KestrelLanguage>> {
    let mut current_parent = root.clone();

    for segment in path_segments {
        let existing_module = current_parent
            .metadata()
            .children()
            .iter()
            .find(|child| {
                child.metadata().kind() == KestrelSymbolKind::Module
                    && child.metadata().name().value == *segment
            })
            .cloned();

        let module_symbol = if let Some(existing) = existing_module {
            existing
        } else {
            let name = Spanned::new(segment.clone(), Span::from(0..segment.len()));
            let span = Span::from(0..segment.len());
            let visibility =
                VisibilityBehavior::new(Some(Visibility::Public), Span::from(0..6), root.clone());

            let module = ModuleSymbol::new(name, span, visibility);
            let module_arc: Arc<dyn Symbol<KestrelLanguage>> = Arc::new(module);

            current_parent.metadata().add_child(&module_arc);

            module_arc
        };

        current_parent = module_symbol;
    }

    current_parent
}

#[derive(Debug)]
struct RootSymbol {
    metadata: SymbolMetadata<KestrelLanguage>,
}

impl RootSymbol {
    fn new() -> Self {
        let name = Spanned::new("<root>".to_string(), Span::from(0..0));

        let metadata = SymbolMetadataBuilder::new(KestrelSymbolKind::Module)
            .with_name(name)
            .with_declaration_span(Span::from(0..0))
            .with_span(Span::from(0..0))
            .build();

        Self { metadata }
    }
}

impl Symbol<KestrelLanguage> for RootSymbol {
    fn metadata(&self) -> &SymbolMetadata<KestrelLanguage> {
        &self.metadata
    }
}
