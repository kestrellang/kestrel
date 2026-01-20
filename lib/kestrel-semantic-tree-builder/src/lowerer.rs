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
    std_auto_import: bool,
}

impl SemanticModelBuilder {
    pub fn new() -> Self {
        Self {
            root: Arc::new(RootSymbol::new()),
            syntax_map: HashMap::new(),
            sources: HashMap::new(),
            std_auto_import: false,
        }
    }

    /// Enable auto-import of standard library modules.
    ///
    /// When enabled, all symbols from std.* modules will be automatically
    /// imported into user source files (non-std modules).
    pub fn enable_std_auto_import(&mut self) {
        self.std_auto_import = true;
    }

    pub fn add_file(
        &mut self,
        file_name: &str,
        syntax: &SyntaxNode,
        source: &str,
        file_id: usize,
        _diagnostics: &mut DiagnosticContext,
    ) {
        let root = self.root.clone();

        let parent_module = match extract_module_path(syntax) {
            Some(path_segments) if !path_segments.is_empty() => {
                build_module_hierarchy(&root, &path_segments, file_id)
            }
            _ => root.clone(),
        };

        let file_name_spanned = Spanned::new(
            file_name.to_string(),
            Span::new(file_id, 0..file_name.len()),
        );
        let source_file_symbol: Arc<dyn Symbol<KestrelLanguage>> = Arc::new(SourceFileSymbol::new(
            file_name_spanned,
            Span::new(file_id, 0..source.len()),
            Some(parent_module.clone()),
        ));

        parent_module.metadata().add_child(&source_file_symbol);

        for child in syntax.children() {
            if child.kind() == kestrel_syntax_tree::SyntaxKind::ModuleDeclaration {
                continue;
            }
            self.walk_node(&child, source, file_id, Some(&source_file_symbol), &root);
        }

        self.sources
            .insert(file_name.to_string(), source.to_string());
    }

    pub fn build(mut self) -> SemanticModel {
        // If auto-import is enabled, inject synthetic imports into user source files
        if self.std_auto_import {
            self.inject_std_imports();
        }

        SemanticModel::new(self.root, self.syntax_map, self.sources)
    }

    /// Inject synthetic wildcard imports for all std.* modules into user source files.
    fn inject_std_imports(&mut self) {
        use kestrel_semantic_tree::symbol::import::{ImportDataBehavior, ImportSymbol};

        // Collect all std.* module paths (including submodules)
        let std_modules = self.collect_std_module_paths(&self.root.clone(), vec![]);

        // Collect all user source files (non-std modules)
        let user_source_files = self.collect_user_source_files(&self.root.clone());

        // For each user source file, inject synthetic imports for all std modules
        for source_file in user_source_files {
            // Get the file_id from the source file's span
            let file_id = source_file.metadata().span().file_id;

            for module_path in &std_modules {
                // Create synthetic import with correct file_id
                let import_name = module_path.join(".");
                let span = Span::synthetic(file_id);
                let name = Spanned::new(import_name.clone(), span.clone());

                let import_symbol = ImportSymbol::new(name, source_file.clone(), span.clone());
                let import_arc: Arc<dyn Symbol<KestrelLanguage>> = Arc::new(import_symbol);

                // Create import data with no items (wildcard import) and no alias
                let module_path_with_spans: Vec<(String, Span)> = module_path
                    .iter()
                    .map(|s| (s.clone(), Span::synthetic(file_id)))
                    .collect();

                let import_data = ImportDataBehavior::new(
                    module_path_with_spans,
                    span,
                    None,  // no alias
                    vec![], // no items = wildcard
                );
                import_arc.metadata().add_behavior(import_data);

                source_file.metadata().add_child(&import_arc);
            }
        }
    }

    /// Recursively collect all module paths that start with "std".
    fn collect_std_module_paths(
        &self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        current_path: Vec<String>,
    ) -> Vec<Vec<String>> {
        let mut result = Vec::new();

        for child in symbol.metadata().children() {
            if child.metadata().kind() == KestrelSymbolKind::Module {
                let name = child.metadata().name().value.clone();
                let mut new_path = current_path.clone();
                new_path.push(name.clone());

                // If this is a std module (path starts with "std"), add it
                if new_path.first().map(|s| s.as_str()) == Some("std") {
                    result.push(new_path.clone());
                }

                // Recursively collect submodules
                let submodule_paths = self.collect_std_module_paths(&child, new_path);
                result.extend(submodule_paths);
            }
        }

        result
    }

    /// Collect all source files that are NOT in std.* modules.
    fn collect_user_source_files(
        &self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    ) -> Vec<Arc<dyn Symbol<KestrelLanguage>>> {
        let mut result = Vec::new();
        self.collect_user_source_files_recursive(symbol, &mut result, false);
        result
    }

    fn collect_user_source_files_recursive(
        &self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        result: &mut Vec<Arc<dyn Symbol<KestrelLanguage>>>,
        in_std: bool,
    ) {
        for child in symbol.metadata().children() {
            match child.metadata().kind() {
                KestrelSymbolKind::Module => {
                    let name = child.metadata().name().value.clone();
                    let is_std = name == "std" || in_std;
                    self.collect_user_source_files_recursive(&child, result, is_std);
                }
                KestrelSymbolKind::SourceFile => {
                    if !in_std {
                        result.push(child.clone());
                    }
                }
                _ => {}
            }
        }
    }

    /// Walk syntax tree and build symbols (iterative to avoid stack overflow on deep trees)
    fn walk_node(
        &mut self,
        syntax: &SyntaxNode,
        source: &str,
        file_id: usize,
        parent: Option<&Arc<dyn Symbol<KestrelLanguage>>>,
        root: &Arc<dyn Symbol<KestrelLanguage>>,
    ) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
        // Use an explicit stack to avoid stack overflow on deeply nested declarations.
        // Each stack entry is (syntax_node, parent_symbol).
        let mut stack: Vec<(SyntaxNode, Option<Arc<dyn Symbol<KestrelLanguage>>>)> =
            vec![(syntax.clone(), parent.cloned())];

        let mut first_result: Option<Arc<dyn Symbol<KestrelLanguage>>> = None;

        while let Some((current_syntax, current_parent)) = stack.pop() {
            let built_symbol = if let Some(builder) = builder_for(current_syntax.kind()) {
                if let Some(symbol) = builder.build_declaration(
                    &current_syntax,
                    source,
                    file_id,
                    current_parent.as_ref(),
                    root,
                ) {
                    self.syntax_map
                        .insert(symbol.metadata().id(), current_syntax.clone());

                    // For field declarations with computed properties, add getter/setter
                    // syntax mappings for the child symbols created by the field builder
                    if current_syntax.kind() == kestrel_syntax_tree::SyntaxKind::FieldDeclaration {
                        self.add_computed_property_syntax_mappings(&symbol, &current_syntax, file_id);
                    }

                    // For subscript declarations, add getter/setter syntax mappings
                    // for the child symbols created by the subscript builder
                    if current_syntax.kind() == kestrel_syntax_tree::SyntaxKind::SubscriptDeclaration {
                        self.add_subscript_syntax_mappings(&symbol, &current_syntax, file_id);
                    }

                    if !builder.is_terminal() {
                        // Add children in reverse order so they're processed left-to-right
                        let children: Vec<_> = current_syntax.children().collect();
                        for child in children.into_iter().rev() {
                            stack.push((child, Some(symbol.clone())));
                        }
                    }

                    // Remember the first symbol built (for return value)
                    if first_result.is_none() {
                        first_result = Some(symbol.clone());
                    }

                    Some(symbol)
                } else {
                    None
                }
            } else {
                None
            };

            // If no symbol was built for this node, still process its children
            if built_symbol.is_none() {
                let children: Vec<_> = current_syntax.children().collect();
                for child in children.into_iter().rev() {
                    stack.push((child, current_parent.clone()));
                }
            }
        }

        first_result
    }
}

impl SemanticModelBuilder {
    /// Add syntax mappings for getter/setter symbols that are children of a field.
    ///
    /// The field builder creates getter/setter symbols as children of the field
    /// for computed properties, but those symbols need their syntax nodes added
    /// to the syntax_map so the binder can access them.
    fn add_computed_property_syntax_mappings(
        &mut self,
        field_symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        field_syntax: &SyntaxNode,
        _file_id: usize,
    ) {
        use kestrel_syntax_tree::SyntaxKind;

        // Find the PropertyAccessors node in the field syntax
        let Some(accessors) = field_syntax
            .children()
            .find(|child| child.kind() == SyntaxKind::PropertyAccessors)
        else {
            return;
        };

        // Get the getter clause syntax (explicit form)
        let getter_clause = accessors
            .children()
            .find(|child| child.kind() == SyntaxKind::GetterClause);

        // Get the shorthand body (CodeBlock directly in PropertyAccessors)
        let shorthand_body = accessors
            .children()
            .find(|child| child.kind() == SyntaxKind::CodeBlock);

        // Get the setter clause syntax
        let setter_clause = accessors
            .children()
            .find(|child| child.kind() == SyntaxKind::SetterClause);

        // Map getter/setter symbols to their syntax nodes
        for child in field_symbol.metadata().children() {
            match child.metadata().kind() {
                KestrelSymbolKind::Getter => {
                    // Prefer explicit GetterClause, fall back to shorthand CodeBlock
                    if let Some(ref getter_syntax) = getter_clause {
                        self.syntax_map
                            .insert(child.metadata().id(), getter_syntax.clone());
                    } else if let Some(ref body_syntax) = shorthand_body {
                        self.syntax_map
                            .insert(child.metadata().id(), body_syntax.clone());
                    }
                }
                KestrelSymbolKind::Setter => {
                    if let Some(ref setter_syntax) = setter_clause {
                        self.syntax_map
                            .insert(child.metadata().id(), setter_syntax.clone());
                    }
                }
                _ => {}
            }
        }
    }

    /// Add syntax mappings for getter/setter symbols that are children of a subscript.
    ///
    /// The subscript builder creates getter/setter symbols as children of the subscript,
    /// but those symbols need their syntax nodes added to the syntax_map so the binder
    /// can access them.
    fn add_subscript_syntax_mappings(
        &mut self,
        subscript_symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        subscript_syntax: &SyntaxNode,
        _file_id: usize,
    ) {
        use kestrel_syntax_tree::SyntaxKind;

        // Find the SubscriptBody node in the subscript syntax
        let Some(body) = subscript_syntax
            .children()
            .find(|child| child.kind() == SyntaxKind::SubscriptBody)
        else {
            return;
        };

        // Check for PropertyAccessors (explicit get/set form)
        let accessors = body
            .children()
            .find(|child| child.kind() == SyntaxKind::PropertyAccessors);

        // Get the getter clause syntax (explicit form)
        let getter_clause = accessors.as_ref().and_then(|acc| {
            acc.children()
                .find(|child| child.kind() == SyntaxKind::GetterClause)
        });

        // Get the shorthand body (CodeBlock directly in SubscriptBody)
        let shorthand_body = if accessors.is_none() {
            body.children()
                .find(|child| child.kind() == SyntaxKind::CodeBlock)
        } else {
            None
        };

        // Get the setter clause syntax
        let setter_clause = accessors.as_ref().and_then(|acc| {
            acc.children()
                .find(|child| child.kind() == SyntaxKind::SetterClause)
        });

        // Map getter/setter symbols to their syntax nodes
        for child in subscript_symbol.metadata().children() {
            match child.metadata().kind() {
                KestrelSymbolKind::Getter => {
                    // Prefer explicit GetterClause, fall back to shorthand CodeBlock
                    if let Some(ref getter_syntax) = getter_clause {
                        self.syntax_map
                            .insert(child.metadata().id(), getter_syntax.clone());
                    } else if let Some(ref body_syntax) = shorthand_body {
                        self.syntax_map
                            .insert(child.metadata().id(), body_syntax.clone());
                    }
                }
                KestrelSymbolKind::Setter => {
                    if let Some(ref setter_syntax) = setter_clause {
                        self.syntax_map
                            .insert(child.metadata().id(), setter_syntax.clone());
                    }
                }
                _ => {}
            }
        }
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
            file.file_id,
            diagnostics,
        );
    }

    builder.build()
}

fn builder_for(
    kind: kestrel_syntax_tree::SyntaxKind,
) -> Option<&'static dyn crate::builder::Builder> {
    use crate::builders::{
        DeinitBuilder, EnumBuilder, EnumCaseBuilder, ExtensionBuilder, FieldBuilder,
        FunctionBuilder, ImportBuilder, InitializerBuilder, ModuleBuilder, ProtocolBuilder,
        StructBuilder, SubscriptBuilder, TerminalBuilder, TypeAliasBuilder,
    };
    use kestrel_syntax_tree::SyntaxKind;

    static DEINIT: DeinitBuilder = DeinitBuilder;
    static ENUM: EnumBuilder = EnumBuilder;
    static ENUM_CASE: EnumCaseBuilder = EnumCaseBuilder;
    static EXTENSION: ExtensionBuilder = ExtensionBuilder;
    static FIELD: FieldBuilder = FieldBuilder;
    static FUNCTION: FunctionBuilder = FunctionBuilder;
    static IMPORT: ImportBuilder = ImportBuilder;
    static INITIALIZER: InitializerBuilder = InitializerBuilder;
    static MODULE: ModuleBuilder = ModuleBuilder;
    static PROTOCOL: ProtocolBuilder = ProtocolBuilder;
    static STRUCT: StructBuilder = StructBuilder;
    static SUBSCRIPT: SubscriptBuilder = SubscriptBuilder;
    static TERMINAL: TerminalBuilder = TerminalBuilder;
    static TYPE_ALIAS: TypeAliasBuilder = TypeAliasBuilder;

    match kind {
        SyntaxKind::ModuleDeclaration => Some(&MODULE),
        SyntaxKind::ImportDeclaration => Some(&IMPORT),
        SyntaxKind::TypeAliasDeclaration => Some(&TYPE_ALIAS),
        SyntaxKind::ProtocolDeclaration => Some(&PROTOCOL),
        SyntaxKind::StructDeclaration => Some(&STRUCT),
        SyntaxKind::EnumDeclaration => Some(&ENUM),
        SyntaxKind::EnumCaseDeclaration => Some(&ENUM_CASE),
        SyntaxKind::ExtensionDeclaration => Some(&EXTENSION),
        SyntaxKind::FieldDeclaration => Some(&FIELD),
        SyntaxKind::FunctionDeclaration => Some(&FUNCTION),
        SyntaxKind::InitializerDeclaration => Some(&INITIALIZER),
        SyntaxKind::DeinitDeclaration => Some(&DEINIT),
        SyntaxKind::SubscriptDeclaration => Some(&SUBSCRIPT),
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
    file_id: usize,
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
            let name = Spanned::new(segment.clone(), Span::new(file_id, 0..segment.len()));
            let span = Span::new(file_id, 0..segment.len());
            let visibility = VisibilityBehavior::new(
                Some(Visibility::Public),
                Span::new(file_id, 0..6),
                root.clone(),
            );

            let module = ModuleSymbol::new(name, span, visibility, Some(current_parent.clone()));
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
        let name = Spanned::new("<root>".to_string(), Span::new(0, 0..0));

        let metadata = SymbolMetadataBuilder::new(KestrelSymbolKind::Module)
            .with_name(name)
            .with_declaration_span(Span::new(0, 0..0))
            .with_span(Span::new(0, 0..0))
            .build();

        Self { metadata }
    }
}

impl Symbol<KestrelLanguage> for RootSymbol {
    fn metadata(&self) -> &SymbolMetadata<KestrelLanguage> {
        &self.metadata
    }
}
