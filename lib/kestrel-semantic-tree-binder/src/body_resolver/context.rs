//! Body resolution context and core resolution functions.
//!
//! This module contains the `BodyResolutionContext` which holds all state needed
//! during body resolution, plus the entry point functions for resolving function bodies.

use std::sync::Arc;

use kestrel_reporting::DiagnosticContext;
use kestrel_semantic_model::SemanticModel;
use kestrel_semantic_tree::behavior::executable::{CodeBlock, ExecutableBehavior};
use kestrel_semantic_tree::behavior::generics::GenericsBehavior;
use kestrel_semantic_tree::expr::LoopId;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::stmt::Statement;
use kestrel_semantic_tree::symbol::function::FunctionSymbol;
use kestrel_semantic_tree::ty::WhereClause;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::resolution::LocalScope;
use kestrel_syntax_tree::utils::get_node_span;

use super::expressions::resolve_expression;
use super::move_tracker::MoveTracker;
use super::statements::{resolve_statement, resolve_variable_declaration};

/// Information about an active loop during body resolution
#[derive(Debug, Clone)]
pub struct LoopInfo {
    /// The unique ID for this loop
    pub loop_id: LoopId,
    /// Optional label name (without the colon)
    pub label: Option<String>,
    /// Span of the label (for diagnostics)
    #[allow(dead_code)]
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
    /// File id for span construction
    pub file_id: usize,
    /// The function symbol ID (for path resolution context)
    pub function_id: SymbolId,
    /// Local scope for variable tracking
    pub local_scope: LocalScope,
    /// Stack of active loops (for break/continue resolution)
    pub(crate) loop_stack: Vec<LoopInfo>,
    /// Next loop ID to assign
    pub(crate) next_loop_id: u32,
    /// Move tracker for non-copyable values
    pub(crate) move_tracker: MoveTracker,
    /// Where clause for the current function (for copyability checking of type parameters)
    where_clause: WhereClause,
}

impl<'a> BodyResolutionContext<'a> {
    /// Create a new body resolution context
    pub fn new(
        model: &'a SemanticModel,
        diagnostics: &'a mut DiagnosticContext,
        source: &'a str,
        file_id: usize,
        function: Arc<FunctionSymbol>,
    ) -> Self {
        let function_id = function.metadata().id();

        // Get the where clause from the function's generics behavior
        let where_clause = function
            .metadata()
            .get_behavior::<GenericsBehavior>()
            .map(|g| g.where_clause().clone())
            .unwrap_or_else(WhereClause::new);

        let local_scope = LocalScope::new(function);
        BodyResolutionContext {
            model,
            diagnostics,
            source,
            file_id,
            function_id,
            local_scope,
            loop_stack: Vec::new(),
            next_loop_id: 0,
            move_tracker: MoveTracker::new(),
            where_clause,
        }
    }

    /// Create a new body resolution context for a non-function symbol (e.g., initializer, deinit)
    ///
    /// This is used when the symbol is not a FunctionSymbol but still needs body resolution.
    pub fn new_with_scope(
        model: &'a SemanticModel,
        diagnostics: &'a mut DiagnosticContext,
        source: &'a str,
        file_id: usize,
        symbol_id: SymbolId,
        local_scope: LocalScope,
        where_clause: Option<WhereClause>,
    ) -> Self {
        BodyResolutionContext {
            model,
            diagnostics,
            source,
            file_id,
            function_id: symbol_id,
            local_scope,
            loop_stack: Vec::new(),
            next_loop_id: 0,
            move_tracker: MoveTracker::new(),
            where_clause: where_clause.unwrap_or_default(),
        }
    }

    /// Get the where clause for the current function
    pub fn where_clause(&self) -> &WhereClause {
        &self.where_clause
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
            },
            Some(label_name) => {
                // Search for labeled loop (from innermost to outermost)
                self.loop_stack
                    .iter()
                    .rev()
                    .find(|info| info.label.as_deref() == Some(label_name))
                    .map(|info| info.loop_id)
            },
        }
    }

    /// Check if we're currently inside a loop.
    pub fn in_loop(&self) -> bool {
        !self.loop_stack.is_empty()
    }

    /// Get mutable access to the move tracker.
    pub fn move_tracker_mut(&mut self) -> &mut MoveTracker {
        &mut self.move_tracker
    }

    /// Get immutable access to the move tracker.
    pub fn move_tracker(&self) -> &MoveTracker {
        &self.move_tracker
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
                // If this is the last child, check if it's a statement-wrapped expression
                // without a semicolon (e.g., an if-expression as the final value)
                if is_last && let Some(expr) = try_extract_yield_expression(child, ctx) {
                    yield_expr = Some(expr);
                    continue;
                }
                if let Some(stmt) = resolve_statement(child, ctx) {
                    statements.push(stmt);
                }
            },
            SyntaxKind::VariableDeclaration => {
                if let Some(stmt) = resolve_variable_declaration(child, ctx) {
                    statements.push(stmt);
                }
            },
            SyntaxKind::Expression => {
                // If this is the last child without a semicolon, it's the yield expression
                // Otherwise it's an expression statement
                if is_last && !has_trailing_semicolon(child) {
                    yield_expr = Some(resolve_expression(child, ctx));
                } else {
                    let expr = resolve_expression(child, ctx);
                    let span = get_node_span(child, ctx.file_id);
                    statements.push(Statement::expr(expr, span));
                }
            },
            // Skip tokens like braces
            _ => {},
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

/// Try to extract a yield expression from a Statement or ExpressionStatement node.
///
/// This handles the case where an expression (like an if-expression with else) is wrapped in
/// a Statement > ExpressionStatement > Expression structure by the parser. If the
/// innermost expression has no trailing semicolon AND can produce a value, it should be
/// treated as a yield expression rather than a statement.
///
/// Returns Some(expression) if this is a yield expression, None otherwise.
fn try_extract_yield_expression(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> Option<kestrel_semantic_tree::expr::Expression> {
    // Don't extract if this node has a semicolon at its level
    if has_trailing_semicolon(node) {
        return None;
    }

    // Look for the inner content
    for child in node.children() {
        match child.kind() {
            SyntaxKind::ExpressionStatement => {
                // Recurse into ExpressionStatement
                return try_extract_yield_expression(&child, ctx);
            },
            SyntaxKind::Expression => {
                // Found the expression wrapper - look inside for the actual expression
                if !has_trailing_semicolon(&child) {
                    // Check if the inner expression can produce a value
                    if can_be_yield_expression(&child) {
                        return Some(resolve_expression(&child, ctx));
                    }
                }
            },
            // Also handle direct expression kinds (ExprIf, ExprMatch without Expression wrapper)
            SyntaxKind::ExprIf => {
                if !has_trailing_semicolon(&child) && has_else_branch(&child) {
                    return Some(resolve_expression(&child, ctx));
                }
            },
            SyntaxKind::ExprMatch => {
                if !has_trailing_semicolon(&child) {
                    return Some(resolve_expression(&child, ctx));
                }
            },
            _ => {},
        }
    }

    None
}

/// Check if a syntax kind could potentially be a yield expression.
#[allow(dead_code)]
fn can_syntax_kind_be_yield(kind: SyntaxKind) -> bool {
    matches!(kind, SyntaxKind::ExprIf | SyntaxKind::ExprMatch)
    // Note: ExprLoop and ExprWhile are NOT included because:
    // - loop without break-with-value returns Never
    // - while always returns () (no else branch concept)
}

/// Check if an expression node can be used as a yield expression.
///
/// This checks structural properties to determine if the expression can produce a value:
/// - If-expressions: only if they have an else branch
/// - Match expressions: always (they're exhaustive)
/// - Other expressions: always
fn can_be_yield_expression(expr_node: &SyntaxNode) -> bool {
    // Look for the actual expression type inside the Expression wrapper
    if let Some(child) = expr_node.children().next() {
        match child.kind() {
            SyntaxKind::ExprIf => {
                // If-expression can be a yield only if it has an else branch
                return has_else_branch(&child);
            },
            SyntaxKind::ExprMatch => {
                // Match expressions are always exhaustive and can be yield expressions
                return true;
            },
            SyntaxKind::ExprLoop | SyntaxKind::ExprWhile => {
                // Loops cannot be yield expressions - they return () or Never
                return false;
            },
            _ => {
                // Other expressions can be yield expressions
                return true;
            },
        }
    }
    // If we found nothing inside, it's probably a simple expression
    true
}

/// Check if an if-expression can be used as a yield expression.
///
/// An if-expression can only be a yield expression if:
/// 1. It has a complete else chain (all branches covered)
/// 2. The branches end with value expressions (not statements like return/let)
///
/// Examples:
/// - `if x { a } else { b }` - YES if a and b are expressions
/// - `if x { a }` - NO, missing else
/// - `if x { return 1 } else { 2 }` - NO, one branch has return statement
/// - `if x { a } else if y { b } else { c }` - YES if all are expressions
fn has_else_branch(if_node: &SyntaxNode) -> bool {
    // Find the then block (CodeBlock after condition)
    let then_block = if_node
        .children()
        .find(|child| child.kind() == SyntaxKind::CodeBlock);

    // Check if then block ends with a value expression
    if let Some(then) = then_block {
        if !block_ends_with_value_expression(&then) {
            return false;
        }
    } else {
        return false;
    }

    // Find the ElseClause
    let else_clause = if_node
        .children()
        .find(|child| child.kind() == SyntaxKind::ElseClause);

    match else_clause {
        None => false, // No else at all
        Some(else_node) => {
            // Check what's inside the else clause
            for child in else_node.children() {
                match child.kind() {
                    SyntaxKind::ExprIf => {
                        // It's an "else if" - recursively check
                        return has_else_branch(&child);
                    },
                    SyntaxKind::CodeBlock => {
                        // It's a final "else { ... }" - check if it ends with a value
                        return block_ends_with_value_expression(&child);
                    },
                    SyntaxKind::Expression => {
                        // The else if might be wrapped in an Expression node
                        // Look inside for ExprIf
                        for inner in child.children() {
                            if inner.kind() == SyntaxKind::ExprIf {
                                return has_else_branch(&inner);
                            }
                        }
                    },
                    _ => {},
                }
            }
            false
        },
    }
}

/// Check if a code block ends with a value expression (not a statement).
///
/// A block ends with a value expression if its last child is an Expression
/// without a trailing semicolon, or a statement-like expression (if/match with else).
fn block_ends_with_value_expression(block: &SyntaxNode) -> bool {
    // Get the last significant child
    let children: Vec<_> = block.children().collect();

    if let Some(last) = children.last() {
        match last.kind() {
            SyntaxKind::Expression => {
                // Check if it's a value expression without semicolon
                !has_trailing_semicolon(last)
            },
            SyntaxKind::Statement | SyntaxKind::ExpressionStatement => {
                // Check if it's a statement-wrapped expression that can be a value
                // Look inside for the expression
                for child in last.children() {
                    match child.kind() {
                        SyntaxKind::ExpressionStatement => {
                            // Recurse
                            for inner in child.children() {
                                if inner.kind() == SyntaxKind::Expression
                                    && !has_trailing_semicolon(&inner)
                                {
                                    return can_be_yield_expression(&inner);
                                }
                            }
                        },
                        SyntaxKind::Expression => {
                            if !has_trailing_semicolon(&child) {
                                return can_be_yield_expression(&child);
                            }
                        },
                        _ => {},
                    }
                }
                false
            },
            SyntaxKind::VariableDeclaration => {
                // let/var statements are not value expressions
                false
            },
            _ => false,
        }
    } else {
        false
    }
}

/// Resolve a function's body and attach ExecutableBehavior to the symbol
pub fn resolve_and_attach_body(
    function_symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    body_syntax: &SyntaxNode,
    model: &SemanticModel,
    diagnostics: &mut DiagnosticContext,
    source: &str,
    file_id: usize,
) {
    let Some(func_sym) = function_symbol.as_ref().downcast_ref::<FunctionSymbol>() else {
        return;
    };

    let local_scope = if let Ok(func) = function_symbol.clone().downcast_arc::<FunctionSymbol>() {
        LocalScope::new(func)
    } else {
        create_local_scope_for_body(function_symbol.clone(), "__body_resolver_temp")
    };

    // Get the where clause from the function's generics behavior
    let where_clause = function_symbol
        .metadata()
        .get_behavior::<GenericsBehavior>()
        .map(|g| g.where_clause().clone());

    let mut ctx = BodyResolutionContext::new_with_scope(
        model,
        diagnostics,
        source,
        file_id,
        function_symbol.metadata().id(),
        local_scope,
        where_clause,
    );

    // Add parameters to local scope first
    // Mutability depends on access mode:
    // - Borrow: immutable (read-only)
    // - Mutating: mutable (read-write, but caller keeps ownership)
    // - Consuming: mutable (takes ownership, can modify)
    for param in func_sym.parameters() {
        use kestrel_semantic_tree::behavior::callable::ParameterAccessMode;
        let param_ty = param.ty.clone();
        let param_name = param.bind_name.value.clone();
        let param_span = param.bind_name.span.clone();
        let is_mutable = match param.access_mode {
            ParameterAccessMode::Borrow => false,
            ParameterAccessMode::Mutating => true,
            ParameterAccessMode::Consuming => true,
        };
        ctx.local_scope
            .bind(param_name, param_ty, is_mutable, param_span);
    }

    resolve_body_and_attach_executable(function_symbol, body_syntax, &mut ctx);
}

pub(crate) fn resolve_body_and_attach_executable(
    function_symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    body_syntax: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) {
    let code_block = resolve_function_body(body_syntax, ctx);
    let executable = ExecutableBehavior::new(code_block);
    function_symbol.metadata().add_behavior(executable);
}

/// Helper to create a LocalScope for body resolution without requiring `Arc<FunctionSymbol>`.
pub(crate) fn create_local_scope_for_body(
    symbol: Arc<dyn Symbol<KestrelLanguage>>,
    temp_name: &str,
) -> LocalScope {
    use kestrel_semantic_tree::behavior::visibility::{Visibility, VisibilityBehavior};
    use kestrel_span::Spanned;

    // Create a dummy function for the LocalScope.
    // The actual local binding will go to this dummy, but that's okay
    // because we're attaching ExecutableBehavior to the real function
    let dummy_span = Span::new(symbol.metadata().span().file_id, 0..0);
    let name = Spanned::new(temp_name.to_string(), dummy_span.clone());
    let visibility = VisibilityBehavior::new(
        Some(Visibility::Private),
        dummy_span.clone(),
        symbol.clone(),
    );
    let dummy_func = Arc::new(FunctionSymbol::new(
        name, dummy_span, visibility, true, true, None,
    ));

    LocalScope::new(dummy_func)
}

/// Backwards-compatible wrapper for older call sites.
#[allow(dead_code)]
fn create_local_scope_from_dyn(symbol: Arc<dyn Symbol<KestrelLanguage>>) -> LocalScope {
    create_local_scope_for_body(symbol, "__body_resolver_temp")
}
