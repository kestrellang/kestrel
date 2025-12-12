//! Module declaration validation
//!
//! Validates that source files have exactly one module declaration
//! and that it appears first in the file.

use kestrel_reporting::DiagnosticContext;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use crate::diagnostics::{
    ModuleNotFirstError, MultipleModuleDeclarationsError, NoModuleDeclarationError,
};

/// Result of successful module validation
pub struct ModuleDeclaration {
    /// The module declaration syntax node
    pub node: SyntaxNode,
    /// The module path segments (e.g., ["Math", "Vector"])
    pub path: Vec<String>,
}

/// Validates module declarations in source files
///
/// According to the spec:
/// - Exactly one module declaration must be present
/// - It must be the first statement in the file
pub struct ModuleValidator<'a> {
    syntax: &'a SyntaxNode,
    diagnostics: &'a mut DiagnosticContext,
    file_id: usize,
}

impl<'a> ModuleValidator<'a> {
    /// Create a new module validator
    pub fn new(
        syntax: &'a SyntaxNode,
        diagnostics: &'a mut DiagnosticContext,
        file_id: usize,
    ) -> Self {
        Self {
            syntax,
            diagnostics,
            file_id,
        }
    }

    /// Validate and extract the module declaration
    ///
    /// Returns `Some(ModuleDeclaration)` if validation passes or partially succeeds.
    /// Returns `None` if there is no module declaration.
    /// Emits diagnostics for any validation errors.
    pub fn validate(&mut self) -> Option<ModuleDeclaration> {
        let module_decls = self.find_module_declarations();

        match module_decls.len() {
            0 => {
                self.emit_no_module_error();
                None
            }
            1 => {
                let module_decl = &module_decls[0];
                self.validate_is_first(module_decl);
                let path = self.extract_path(module_decl);
                Some(ModuleDeclaration {
                    node: module_decl.clone(),
                    path,
                })
            }
            _ => {
                self.emit_multiple_modules_error(&module_decls);
                // Return first module to allow partial processing
                let path = self.extract_path(&module_decls[0]);
                Some(ModuleDeclaration {
                    node: module_decls[0].clone(),
                    path,
                })
            }
        }
    }

    /// Find all module declarations in the syntax tree
    fn find_module_declarations(&self) -> Vec<SyntaxNode> {
        self.syntax
            .children()
            .filter(|child| child.kind() == SyntaxKind::ModuleDeclaration)
            .collect()
    }

    /// Check if the module declaration is the first item in the file
    fn validate_is_first(&mut self, module_decl: &SyntaxNode) {
        let first_child = self.syntax.children().next();

        if let Some(first) = first_child {
            if first.kind() != SyntaxKind::ModuleDeclaration {
                let first_start: usize = first.text_range().start().into();
                let first_end: usize = first.text_range().end().into();
                let module_start: usize = module_decl.text_range().start().into();
                let module_end: usize = module_decl.text_range().end().into();

                let error = ModuleNotFirstError {
                    module_span: Span::from(module_start..module_end),
                    first_item_span: Span::from(first_start..first_end),
                    first_item_kind: format!("{:?}", first.kind()),
                };
                self.diagnostics.throw(error);
            }
        }
    }

    /// Extract the module path from a module declaration node
    fn extract_path(&self, module_decl: &SyntaxNode) -> Vec<String> {
        let module_path_node = module_decl
            .children()
            .find(|child| child.kind() == SyntaxKind::ModulePath)
            .expect("ModuleDeclaration must have ModulePath child");

        module_path_node
            .children_with_tokens()
            .filter_map(|elem| elem.into_token())
            .filter(|tok| tok.kind() == SyntaxKind::Identifier)
            .map(|tok| tok.text().to_string())
            .collect()
    }

    /// Emit error for missing module declaration
    fn emit_no_module_error(&mut self) {
        let span = if let Some(first_decl) = self.syntax.children().next() {
            let start: usize = first_decl.text_range().start().into();
            let end: usize = first_decl.text_range().end().into();
            Span::from(start..end.min(start + 1))
        } else {
            Span::from(0..1)
        };

        let error = NoModuleDeclarationError { span };
        self.diagnostics.throw(error);
    }

    /// Emit error for multiple module declarations
    fn emit_multiple_modules_error(&mut self, module_decls: &[SyntaxNode]) {
        let first_decl = &module_decls[0];
        let first_start: usize = first_decl.text_range().start().into();
        let first_end: usize = first_decl.text_range().end().into();

        let duplicate_spans: Vec<Span> = module_decls
            .iter()
            .skip(1)
            .map(|decl| {
                let start: usize = decl.text_range().start().into();
                let end: usize = decl.text_range().end().into();
                Span::from(start..end)
            })
            .collect();

        let error = MultipleModuleDeclarationsError {
            first_span: Span::from(first_start..first_end),
            duplicate_spans,
            count: module_decls.len(),
        };
        self.diagnostics.throw(error);
    }
}
