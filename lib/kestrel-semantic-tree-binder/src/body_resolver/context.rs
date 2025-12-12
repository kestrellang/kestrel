//! Body resolution context and core resolution functions.
//!
//! This module contains the `BodyResolutionContext` which holds all state needed
//! during body resolution, plus the entry point functions for resolving function bodies.

use std::sync::Arc;

use kestrel_reporting::DiagnosticContext;
use kestrel_semantic_model::SemanticModel;
use kestrel_semantic_tree::behavior::executable::{CodeBlock, ExecutableBehavior};
use kestrel_semantic_tree::expr::LoopId;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::stmt::Statement;
use kestrel_semantic_tree::symbol::function::FunctionSymbol;
use kestrel_semantic_tree::ty::Ty;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::resolution::LocalScope;
use kestrel_syntax_tree::utils::get_node_span;

use super::expressions::resolve_expression;
use super::statements::{resolve_statement, resolve_variable_declaration};

/// Information about an active loop during body resolution
#[derive(Debug, Clone)]
pub struct LoopInfo {
    /// The unique ID for this loop
    pub loop_id: LoopId,
    /// Optional label name (without the colon)
    pub label: Option<String>,
    /// Span of the label (for diagnostics)
    pub label_span: Option<Span>,
}

/// Context for body resolution
pub struct BodyResolutionContext<'a> {
    /// The semantic model for queries
    pub model: &'a SemanticModel,
    /// Diagnostics collector
    pub diagnostics: &'a mut DiagnosticContext,
    /// Source code for span extraction
    pub source: &'a str,
    /// The function symbol ID (for path resolution context)
    pub function_id: SymbolId,
    /// Local scope for variable tracking
    pub local_scope: LocalScope,
    /// Stack of active loops (for break/continue resolution)
    pub(crate) loop_stack: Vec<LoopInfo>,
    /// Next loop ID to assign
    pub(crate) next_loop_id: u32,
}

impl<'a> BodyResolutionContext<'a> {
    /// Create a new body resolution context
    pub fn new(
        model: &'a SemanticModel,
        diagnostics: &'a mut DiagnosticContext,
        source: &'a str,
        function: Arc<FunctionSymbol>,
    ) -> Self {
        let function_id = function.metadata().id();
        let local_scope = LocalScope::new(function);
        BodyResolutionContext {
            model,
            diagnostics,
            source,
            function_id,
            local_scope,
            loop_stack: Vec::new(),
            next_loop_id: 0,
        }
    }

    /// Enter a loop, returning its LoopId.
    pub fn enter_loop(&mut self, label: Option<String>, label_span: Option<Span>) -> LoopId {
        let loop_id = LoopId::new(self.next_loop_id);
        self.next_loop_id += 1;
        self.loop_stack.push(LoopInfo {
            loop_id,
            label,
            label_span,
        });
        loop_id
    }

    /// Exit the current loop.
    pub fn exit_loop(&mut self) {
        self.loop_stack.pop();
    }

    /// Find the target loop for a break/continue.
    ///
    /// If `label` is Some, searches for a loop with that label.
    /// If `label` is None, returns the innermost loop.
    /// Returns None if no matching loop is found.
    pub fn find_loop(&self, label: Option<&str>) -> Option<LoopId> {
        match label {
            None => {
                // Return innermost loop
                self.loop_stack.last().map(|info| info.loop_id)
            }
            Some(label_name) => {
                // Search for labeled loop (from innermost to outermost)
                self.loop_stack
                    .iter()
                    .rev()
                    .find(|info| info.label.as_deref() == Some(label_name))
                    .map(|info| info.loop_id)
            }
        }
    }

    /// Check if we're currently inside a loop.
    pub fn in_loop(&self) -> bool {
        !self.loop_stack.is_empty()
    }
}

/// Resolve a function body syntax node into a CodeBlock
pub fn resolve_function_body(body_node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> CodeBlock {
    // FunctionBody contains either a CodeBlock or a single Expression
    // FunctionBody -> { CodeBlock } | { Expression }

    // Check for CodeBlock child
    if let Some(code_block) = body_node
        .children()
        .find(|c| c.kind() == SyntaxKind::CodeBlock)
    {
        return resolve_code_block(&code_block, ctx);
    }

    // Check for Expression child (shorthand: func foo() -> Int = 42)
    if let Some(expr_node) = body_node
        .children()
        .find(|c| c.kind() == SyntaxKind::Expression)
    {
        let expr = resolve_expression(&expr_node, ctx);
        return CodeBlock::new(vec![], Some(expr));
    }

    // Empty body
    CodeBlock::empty()
}

/// Resolve a code block (statements + optional yield expression)
pub fn resolve_code_block(block_node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> CodeBlock {
    ctx.local_scope.push_scope();

    let mut statements = Vec::new();
    let mut yield_expr = None;

    // Process children
    let children: Vec<_> = block_node.children().collect();

    for (i, child) in children.iter().enumerate() {
        let is_last = i == children.len() - 1;

        match child.kind() {
            SyntaxKind::Statement | SyntaxKind::ExpressionStatement => {
                if let Some(stmt) = resolve_statement(child, ctx) {
                    statements.push(stmt);
                }
            }
            SyntaxKind::VariableDeclaration => {
                if let Some(stmt) = resolve_variable_declaration(child, ctx) {
                    statements.push(stmt);
                }
            }
            SyntaxKind::Expression => {
                // If this is the last child without a semicolon, it's the yield expression
                // Otherwise it's an expression statement
                if is_last && !has_trailing_semicolon(child) {
                    yield_expr = Some(resolve_expression(child, ctx));
                } else {
                    let expr = resolve_expression(child, ctx);
                    let span = get_node_span(child, ctx.source);
                    statements.push(Statement::expr(expr, span));
                }
            }
            // Skip tokens like braces
            _ => {}
        }
    }

    ctx.local_scope.pop_scope();

    CodeBlock::new(statements, yield_expr)
}

/// Check if a node has a trailing semicolon
fn has_trailing_semicolon(node: &SyntaxNode) -> bool {
    // Check if the node or its parent has a semicolon token after
    node.children_with_tokens()
        .any(|elem| elem.kind() == SyntaxKind::Semicolon)
}

/// Resolve a function's body and attach ExecutableBehavior to the symbol
pub fn resolve_and_attach_body(
    function_symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    body_syntax: &SyntaxNode,
    model: &SemanticModel,
    diagnostics: &mut DiagnosticContext,
    source: &str,
) {
    use kestrel_semantic_model::SymbolFor;

    // Verify it can be downcast to FunctionSymbol
    if function_symbol
        .as_ref()
        .downcast_ref::<FunctionSymbol>()
        .is_none()
    {
        return;
    }

    // Create a new Arc for the function (we need to create LocalScope)
    // Since we already have a FunctionSymbol reference, we need to work around this
    // by getting it from the model
    let Some(func_arc) = model.query(SymbolFor {
        id: function_symbol.metadata().id(),
    }) else {
        return;
    };

    let Some(func_sym_arc) = func_arc.as_ref().downcast_ref::<FunctionSymbol>() else {
        return;
    };

    let mut ctx = BodyResolutionContext {
        model,
        diagnostics,
        source,
        function_id: function_symbol.metadata().id(),
        local_scope: create_local_scope_from_dyn(func_arc.clone()),
        loop_stack: Vec::new(),
        next_loop_id: 0,
    };

    // Add parameters to local scope first
    for param in func_sym_arc.parameters() {
        let param_ty = param.ty.clone();
        let param_name = param.bind_name.value.clone();
        let param_span = param.bind_name.span.clone();
        ctx.local_scope
            .bind(param_name, param_ty, false, param_span);
    }

    // Resolve the body
    let code_block = resolve_function_body(body_syntax, &mut ctx);

    // Create and attach ExecutableBehavior
    let executable = ExecutableBehavior::new(code_block);
    function_symbol.metadata().add_behavior(executable);
}

/// Helper to create LocalScope from Arc<dyn Symbol>
fn create_local_scope_from_dyn(symbol: Arc<dyn Symbol<KestrelLanguage>>) -> LocalScope {
    use kestrel_semantic_tree::behavior::visibility::{Visibility, VisibilityBehavior};
    use kestrel_span::Spanned;

    // Try to downcast - if it fails, create a dummy
    if let Some(_) = symbol.as_ref().downcast_ref::<FunctionSymbol>() {
        // We verified it's a FunctionSymbol, but we can't easily get Arc<FunctionSymbol>
        // from Arc<dyn Symbol>. The proper solution would be to use type_id and unsafe,
        // but for now let's create a new wrapper.
    }

    // Fallback: create a dummy function for the LocalScope
    // The actual local binding will go to this dummy, but that's okay
    // because we're attaching ExecutableBehavior to the real function
    let name = Spanned::new("__body_resolver_temp".to_string(), Span::from(0..0));
    let visibility =
        VisibilityBehavior::new(Some(Visibility::Private), Span::from(0..0), symbol.clone());
    let return_type = Ty::unit(Span::from(0..0));
    let dummy_func = Arc::new(FunctionSymbol::new(
        name,
        Span::from(0..0),
        visibility,
        true,
        true,
        None,
    ));

    LocalScope::new(dummy_func)
}
