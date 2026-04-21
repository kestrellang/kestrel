//! Position-based lookup utilities for LSP features.
//!
//! Provides functions to find symbols and type information at a given source position.

use kestrel_compiler::Compilation;
use kestrel_semantic_tree::behavior::callable::CallableBehavior;
use kestrel_semantic_tree::behavior::executable::{
    CodeBlock, ExecutableBehavior, ResolvedExecutableBehavior,
};
use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_semantic_tree::behavior::valued::ValueBehavior;
use kestrel_semantic_tree::expr::{CallArgument, ExprKind, Expression};
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::stmt::StatementKind;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::TyKind;
use semantic_tree::symbol::{Symbol, SymbolId};
use std::sync::Arc;

/// Information about a symbol at a position.
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    /// The symbol's name
    pub name: String,
    /// The symbol's kind (e.g., "function", "struct", "variable")
    pub kind: String,
    /// Type signature or description
    pub signature: String,
    /// Definition location (file_id, start, end)
    pub definition: Option<(usize, usize, usize)>,
}

/// Find the symbol at a given byte offset in a file.
///
/// Returns information about the symbol if found.
pub fn find_symbol_at_position(
    compilation: &Compilation,
    file_id: usize,
    offset: usize,
) -> Option<SymbolInfo> {
    let model = compilation.semantic_model()?;

    // Walk all symbols looking for one whose name span contains the offset
    fn find_in_symbol(
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        file_id: usize,
        offset: usize,
    ) -> Option<SymbolInfo> {
        let metadata = symbol.metadata();
        let name_span = &metadata.name().span;

        // Check if the cursor is on this symbol's name
        if name_span.file_id == file_id && offset >= name_span.start && offset <= name_span.end {
            let kind = format!("{:?}", metadata.kind());
            let signature = build_signature(symbol);
            let decl_span = metadata.declaration_span();

            return Some(SymbolInfo {
                name: metadata.name().value.clone(),
                kind,
                signature,
                definition: Some((decl_span.file_id, decl_span.start, decl_span.end)),
            });
        }

        // Check children
        for child in metadata.children() {
            if let Some(info) = find_in_symbol(&child, file_id, offset) {
                return Some(info);
            }
        }

        None
    }

    // Search from root
    let root = model.root();
    for child in root.metadata().children() {
        if let Some(info) = find_in_symbol(&child, file_id, offset) {
            return Some(info);
        }
    }

    None
}

/// Build a signature string for a symbol.
fn build_signature(symbol: &Arc<dyn Symbol<KestrelLanguage>>) -> String {
    let metadata = symbol.metadata();
    let kind = format!("{:?}", metadata.kind()).to_lowercase();
    let name = &metadata.name().value;

    // Check for callable behavior (functions, methods)
    if let Some(callable) = metadata.get_behavior::<CallableBehavior>() {
        let params: Vec<String> = callable
            .parameters()
            .iter()
            .map(|p| {
                let label = p.external_label().unwrap_or("_");
                if label == "_" {
                    format!("{}", p.ty)
                } else {
                    format!("{}: {}", label, p.ty)
                }
            })
            .collect();
        return format!(
            "func {}({}) -> {}",
            name,
            params.join(", "),
            callable.return_type()
        );
    }

    // Check for typed behavior (fields, constants)
    if let Some(typed) = metadata.get_behavior::<TypedBehavior>() {
        return format!("{} {}: {}", kind, name, typed.ty());
    }

    // Check for valued behavior (variables)
    if let Some(valued) = metadata.get_behavior::<ValueBehavior>() {
        return format!("{} {}: {}", kind, name, valued.ty());
    }

    // Default: just kind and name
    format!("{} {}", kind, name)
}

/// Information about a function call site for signature help.
#[derive(Debug, Clone)]
pub struct CallSiteInfo {
    /// The function name being called
    pub function_name: String,
    /// The function's parameters
    pub parameters: Vec<ParameterInfo>,
    /// The return type
    pub return_type: String,
    /// Which parameter the cursor is on (0-indexed)
    pub active_parameter: usize,
}

/// Information about a function parameter.
#[derive(Debug, Clone)]
pub struct ParameterInfo {
    /// The parameter label (e.g., "x: Int")
    pub label: String,
    /// Optional documentation
    pub documentation: Option<String>,
}

/// Find the function being called at a given position (for signature help).
///
/// Uses the semantic model's resolved expression tree to find the innermost call
/// expression containing the cursor, then extracts parameter info from the callee's
/// `CallableBehavior`. Falls back to text-scanning if the semantic model isn't available.
pub fn find_call_site_at_position(
    compilation: &Compilation,
    source: &str,
    file_id: usize,
    offset: usize,
) -> Option<CallSiteInfo> {
    // Try semantic model approach first
    if let Some(result) = find_call_site_from_model(compilation, file_id, offset) {
        return Some(result);
    }

    // Fall back to text-scanning heuristic
    find_call_site_from_text(compilation, source, offset)
}

/// Semantic model approach: walk resolved expression trees to find the call at cursor.
fn find_call_site_from_model(
    compilation: &Compilation,
    file_id: usize,
    offset: usize,
) -> Option<CallSiteInfo> {
    let model = compilation.semantic_model()?;

    // Find the function symbol whose body contains the cursor
    let enclosing = find_enclosing_function(model.root(), file_id, offset)?;
    let metadata = enclosing.metadata();

    // Get the resolved body (post-inference), falling back to pre-inference body
    let body = metadata
        .get_behavior::<ResolvedExecutableBehavior>()
        .map(|b| b.body().clone())
        .or_else(|| {
            metadata
                .get_behavior::<ExecutableBehavior>()
                .map(|b| b.body().clone())
        })?;

    // Find the innermost call expression at the cursor position
    let call_info = find_innermost_call(&body, file_id, offset)?;

    // Look up callee's CallableBehavior from the registry
    let registry = model.registry();
    build_call_site_info(&call_info, registry, offset)
}

/// Information about a call expression found in the expression tree.
struct CallExprInfo {
    /// The callee symbol ID, if resolvable
    callee_id: Option<SymbolId>,
    /// The function name
    name: String,
    /// The arguments with their spans
    arguments: Vec<CallArgument>,
}

/// Find the function symbol whose span contains the given offset.
fn find_enclosing_function(
    root: &Arc<dyn Symbol<KestrelLanguage>>,
    file_id: usize,
    offset: usize,
) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
    fn search(
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        file_id: usize,
        offset: usize,
    ) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
        let metadata = symbol.metadata();
        let span = metadata.span();

        if span.file_id != file_id || offset < span.start || offset > span.end {
            return None;
        }

        // Check children first for more specific (inner) functions
        for child in metadata.children() {
            if let Some(found) = search(&child, file_id, offset) {
                return Some(found);
            }
        }

        // This symbol contains the offset — check if it has an executable body
        if metadata
            .get_behavior::<ResolvedExecutableBehavior>()
            .is_some()
            || metadata.get_behavior::<ExecutableBehavior>().is_some()
        {
            return Some(symbol.clone());
        }

        None
    }

    for child in root.metadata().children() {
        if let Some(found) = search(&child, file_id, offset) {
            return Some(found);
        }
    }
    None
}

/// Walk the expression tree to find the innermost call containing the cursor.
fn find_innermost_call(body: &CodeBlock, file_id: usize, offset: usize) -> Option<CallExprInfo> {
    let mut best: Option<CallExprInfo> = None;

    visit_code_block(body, file_id, offset, &mut best);

    best
}

/// Visit a code block, looking for call expressions containing the offset.
fn visit_code_block(
    block: &CodeBlock,
    file_id: usize,
    offset: usize,
    best: &mut Option<CallExprInfo>,
) {
    for stmt in &block.statements {
        visit_statement(stmt, file_id, offset, best);
    }
    if let Some(expr) = block.yield_expr() {
        visit_expr(expr, file_id, offset, best);
    }
}

/// Visit a statement, looking for call expressions.
fn visit_statement(
    stmt: &kestrel_semantic_tree::stmt::Statement,
    file_id: usize,
    offset: usize,
    best: &mut Option<CallExprInfo>,
) {
    match &stmt.kind {
        StatementKind::Binding {
            value: Some(expr), ..
        } => {
            visit_expr(expr, file_id, offset, best);
        },
        StatementKind::Expr(expr) => {
            visit_expr(expr, file_id, offset, best);
        },
        StatementKind::GuardLet {
            conditions,
            else_block,
        } => {
            visit_conditions(conditions, file_id, offset, best);
            visit_code_block(else_block, file_id, offset, best);
        },
        _ => {},
    }
}

/// Visit conditions (if-let chains, guard-let chains).
fn visit_conditions(
    conditions: &[kestrel_semantic_tree::expr::IfCondition],
    file_id: usize,
    offset: usize,
    best: &mut Option<CallExprInfo>,
) {
    for cond in conditions {
        match cond {
            kestrel_semantic_tree::expr::IfCondition::Expr(expr) => {
                visit_expr(expr, file_id, offset, best);
            },
            kestrel_semantic_tree::expr::IfCondition::Let { value, .. } => {
                visit_expr(value, file_id, offset, best);
            },
        }
    }
}

/// Visit statements in a Vec.
fn visit_statements(
    stmts: &[kestrel_semantic_tree::stmt::Statement],
    file_id: usize,
    offset: usize,
    best: &mut Option<CallExprInfo>,
) {
    for stmt in stmts {
        visit_statement(stmt, file_id, offset, best);
    }
}

/// Visit an expression, recording call expressions and recursing into children.
fn visit_expr(expr: &Expression, file_id: usize, offset: usize, best: &mut Option<CallExprInfo>) {
    let span = &expr.span;
    let contains_cursor = span.file_id == file_id && offset >= span.start && offset <= span.end;

    if !contains_cursor {
        return;
    }

    // Check if this expression is a call — if so, record it as a candidate.
    // We always take the latest (innermost) match since we recurse into children after.
    if let Some(info) = extract_call_info(expr) {
        *best = Some(info);
    }

    // Recurse into children to find a more specific (inner) call
    match &expr.kind {
        // Calls — recurse into arguments (inner calls like `foo(bar(` )
        ExprKind::Call {
            callee, arguments, ..
        } => {
            visit_expr(callee, file_id, offset, best);
            visit_arguments(arguments, file_id, offset, best);
        },
        ExprKind::DeferredMethodCall {
            receiver,
            arguments,
            ..
        } => {
            visit_expr(receiver, file_id, offset, best);
            visit_arguments(arguments, file_id, offset, best);
        },
        ExprKind::DeferredStaticCall { arguments, .. } => {
            visit_arguments(arguments, file_id, offset, best);
        },
        ExprKind::DeferredInitCall { arguments, .. } => {
            visit_arguments(arguments, file_id, offset, best);
        },
        ExprKind::DeferredFunctionCall { arguments, .. } => {
            visit_arguments(arguments, file_id, offset, best);
        },
        ExprKind::DeferredSubscriptCall {
            receiver,
            arguments,
        } => {
            visit_expr(receiver, file_id, offset, best);
            visit_arguments(arguments, file_id, offset, best);
        },
        ExprKind::ImplicitStructInit { arguments, .. } => {
            visit_arguments(arguments, file_id, offset, best);
        },
        ExprKind::DelegatingInit { arguments, .. } => {
            visit_arguments(arguments, file_id, offset, best);
        },
        ExprKind::SubscriptCall {
            receiver,
            arguments,
            ..
        } => {
            visit_expr(receiver, file_id, offset, best);
            visit_arguments(arguments, file_id, offset, best);
        },
        ExprKind::PrimitiveMethodCall {
            receiver,
            arguments,
            ..
        } => {
            visit_expr(receiver, file_id, offset, best);
            visit_arguments(arguments, file_id, offset, best);
        },
        ExprKind::LangIntrinsic { arguments, .. } => {
            visit_arguments(arguments, file_id, offset, best);
        },
        ExprKind::ImplicitMemberAccess {
            arguments: Some(arguments),
            ..
        } => {
            visit_arguments(arguments, file_id, offset, best);
        },

        // Non-call expressions with children
        ExprKind::FieldAccess { object, .. }
        | ExprKind::ProtocolPropertyAccess {
            receiver: object, ..
        }
        | ExprKind::TupleIndex { tuple: object, .. }
        | ExprKind::MethodRef {
            receiver: object, ..
        }
        | ExprKind::PrimitiveMethodRef {
            receiver: object, ..
        }
        | ExprKind::DeferredMemberAccess {
            receiver: object, ..
        }
        | ExprKind::Throw { value: object } => {
            visit_expr(object, file_id, offset, best);
        },
        ExprKind::Grouping(inner) => {
            visit_expr(inner, file_id, offset, best);
        },
        ExprKind::Assignment { target, value } => {
            visit_expr(target, file_id, offset, best);
            visit_expr(value, file_id, offset, best);
        },
        ExprKind::Return { value: Some(v) } => {
            visit_expr(v, file_id, offset, best);
        },
        ExprKind::Array(elems) | ExprKind::Tuple(elems) => {
            for e in elems {
                visit_expr(e, file_id, offset, best);
            }
        },
        ExprKind::Dictionary(pairs) => {
            for (k, v) in pairs {
                visit_expr(k, file_id, offset, best);
                visit_expr(v, file_id, offset, best);
            }
        },
        ExprKind::InterpolatedString { parts } => {
            for part in parts {
                if let kestrel_semantic_tree::expr::InterpolationPart::Interpolation {
                    expr: e,
                    ..
                } = part
                {
                    visit_expr(e, file_id, offset, best);
                }
            }
        },
        ExprKind::If {
            conditions,
            then_branch,
            then_value,
            else_branch,
        } => {
            visit_conditions(conditions, file_id, offset, best);
            visit_statements(then_branch, file_id, offset, best);
            if let Some(v) = then_value {
                visit_expr(v, file_id, offset, best);
            }
            if let Some(eb) = else_branch {
                match eb {
                    kestrel_semantic_tree::expr::ElseBranch::Block { statements, value } => {
                        visit_statements(statements, file_id, offset, best);
                        if let Some(v) = value {
                            visit_expr(v, file_id, offset, best);
                        }
                    },
                    kestrel_semantic_tree::expr::ElseBranch::ElseIf(e) => {
                        visit_expr(e, file_id, offset, best);
                    },
                }
            }
        },
        ExprKind::While {
            condition, body, ..
        } => {
            visit_expr(condition, file_id, offset, best);
            visit_statements(body, file_id, offset, best);
        },
        ExprKind::WhileLet {
            conditions, body, ..
        } => {
            visit_conditions(conditions, file_id, offset, best);
            visit_statements(body, file_id, offset, best);
        },
        ExprKind::Loop { body, .. } => {
            visit_statements(body, file_id, offset, best);
        },
        ExprKind::Match { scrutinee, arms } => {
            visit_expr(scrutinee, file_id, offset, best);
            for arm in arms {
                if let Some(g) = &arm.guard {
                    visit_expr(g, file_id, offset, best);
                }
                visit_expr(&arm.body, file_id, offset, best);
            }
        },
        ExprKind::Block { statements, value } => {
            visit_statements(statements, file_id, offset, best);
            if let Some(v) = value {
                visit_expr(v, file_id, offset, best);
            }
        },
        ExprKind::Closure {
            body, tail_expr, ..
        } => {
            visit_statements(body, file_id, offset, best);
            if let Some(e) = tail_expr {
                visit_expr(e, file_id, offset, best);
            }
        },

        // Leaf expressions — no children to recurse into
        _ => {},
    }
}

/// Visit call arguments.
fn visit_arguments(
    args: &[CallArgument],
    file_id: usize,
    offset: usize,
    best: &mut Option<CallExprInfo>,
) {
    for arg in args {
        visit_expr(&arg.value, file_id, offset, best);
    }
}

/// Extract call info from a call expression.
fn extract_call_info(expr: &Expression) -> Option<CallExprInfo> {
    match &expr.kind {
        ExprKind::Call {
            callee, arguments, ..
        } => {
            let (callee_id, name) = extract_callee_info(callee);
            Some(CallExprInfo {
                callee_id,
                name,
                arguments: arguments.clone(),
            })
        },
        ExprKind::DeferredMethodCall {
            receiver: _,
            method_name,
            arguments,
            ..
        } => Some(CallExprInfo {
            callee_id: None,
            name: method_name.clone(),
            arguments: arguments.clone(),
        }),
        ExprKind::DeferredStaticCall {
            method_name,
            arguments,
            ..
        } => Some(CallExprInfo {
            callee_id: None,
            name: method_name.clone(),
            arguments: arguments.clone(),
        }),
        ExprKind::DeferredInitCall {
            struct_ty,
            arguments,
            ..
        } => Some(CallExprInfo {
            callee_id: None,
            name: format!("{}", struct_ty),
            arguments: arguments.clone(),
        }),
        ExprKind::DeferredFunctionCall {
            candidates,
            arguments,
            ..
        } => {
            let id = candidates.first().copied();
            Some(CallExprInfo {
                callee_id: id,
                name: String::new(), // will be filled from symbol
                arguments: arguments.clone(),
            })
        },
        ExprKind::ImplicitStructInit {
            struct_type,
            arguments,
        } => Some(CallExprInfo {
            callee_id: None,
            name: format!("{}", struct_type),
            arguments: arguments.clone(),
        }),
        ExprKind::SubscriptCall {
            getter, arguments, ..
        } => Some(CallExprInfo {
            callee_id: Some(*getter),
            name: "subscript".to_string(),
            arguments: arguments.clone(),
        }),
        ExprKind::DeferredSubscriptCall { arguments, .. } => Some(CallExprInfo {
            callee_id: None,
            name: "subscript".to_string(),
            arguments: arguments.clone(),
        }),
        ExprKind::DelegatingInit {
            initializer,
            arguments,
            ..
        } => Some(CallExprInfo {
            callee_id: Some(*initializer),
            name: "init".to_string(),
            arguments: arguments.clone(),
        }),
        ExprKind::PrimitiveMethodCall {
            method, arguments, ..
        } => Some(CallExprInfo {
            callee_id: None,
            name: method.name().to_string(),
            arguments: arguments.clone(),
        }),
        _ => None,
    }
}

/// Extract callee symbol ID and name from a Call's callee expression.
fn extract_callee_info(callee: &Expression) -> (Option<SymbolId>, String) {
    match &callee.kind {
        ExprKind::SymbolRef(id) => (Some(*id), String::new()),
        ExprKind::TypeRef(id) => (Some(*id), String::new()),
        ExprKind::OverloadedRef(ids) => (ids.first().copied(), String::new()),
        ExprKind::MethodRef {
            candidates,
            method_name,
            ..
        } => (candidates.first().copied(), method_name.clone()),
        _ => (None, String::new()),
    }
}

/// Determine active parameter from argument spans and cursor offset.
fn active_parameter_from_args(arguments: &[CallArgument], offset: usize) -> usize {
    if arguments.is_empty() {
        return 0;
    }

    // Find which argument the cursor is in or after
    for (i, arg) in arguments.iter().enumerate() {
        if offset <= arg.span.end {
            return i;
        }
    }

    // Cursor is after all arguments
    arguments.len()
}

/// Build a `CallSiteInfo` from a `CallExprInfo` using the symbol registry.
fn build_call_site_info(
    info: &CallExprInfo,
    registry: &kestrel_semantic_model::SymbolRegistry,
    offset: usize,
) -> Option<CallSiteInfo> {
    // Try to look up the callee symbol for parameter info
    if let Some(id) = info.callee_id {
        if let Some(symbol) = registry.get(id) {
            let metadata = symbol.metadata();

            // Get name from symbol if we don't have one
            let name = if info.name.is_empty() {
                metadata.name().value.clone()
            } else {
                info.name.clone()
            };

            if let Some(callable) = metadata.get_behavior::<CallableBehavior>() {
                let parameters = build_parameter_list(&callable);
                let active = active_parameter_from_args(&info.arguments, offset);

                return Some(CallSiteInfo {
                    function_name: name.clone(),
                    parameters: parameters.clone(),
                    return_type: callable.return_type().to_string(),
                    active_parameter: active.min(parameters.len().saturating_sub(1)),
                });
            }

            // For struct types (init calls), build params from fields
            if metadata.kind() == KestrelSymbolKind::Struct {
                let parameters = build_struct_field_params(&symbol);
                let active = active_parameter_from_args(&info.arguments, offset);
                return Some(CallSiteInfo {
                    function_name: name.clone(),
                    parameters: parameters.clone(),
                    return_type: name,
                    active_parameter: active.min(parameters.len().saturating_sub(1)),
                });
            }
        }
    }

    // No symbol found — still return what we know from the expression
    if !info.name.is_empty() {
        let active = active_parameter_from_args(&info.arguments, offset);
        // Build parameter placeholders from arguments
        let parameters: Vec<ParameterInfo> = info
            .arguments
            .iter()
            .enumerate()
            .map(|(i, arg)| {
                let label = if let Some(lbl) = &arg.label {
                    format!("{}: {}", lbl, arg.value.ty)
                } else {
                    format!("_{}: {}", i, arg.value.ty)
                };
                ParameterInfo {
                    label,
                    documentation: None,
                }
            })
            .collect();

        return Some(CallSiteInfo {
            function_name: info.name.clone(),
            parameters: parameters.clone(),
            return_type: format!("{}", expr_type_display(&info.arguments)),
            active_parameter: active.min(if parameters.is_empty() {
                0
            } else {
                parameters.len() - 1
            }),
        });
    }

    None
}

/// Build parameter list from a CallableBehavior.
fn build_parameter_list(callable: &CallableBehavior) -> Vec<ParameterInfo> {
    callable
        .parameters()
        .iter()
        .map(|p| {
            let label = if let Some(ext) = p.external_label() {
                if ext == "_" {
                    format!("{}: {}", p.bind_name.value, p.ty)
                } else {
                    format!("{}: {}", ext, p.ty)
                }
            } else {
                format!("{}: {}", p.bind_name.value, p.ty)
            };
            ParameterInfo {
                label,
                documentation: None,
            }
        })
        .collect()
}

/// Build parameter list from struct fields (for implicit memberwise init).
fn build_struct_field_params(symbol: &Arc<dyn Symbol<KestrelLanguage>>) -> Vec<ParameterInfo> {
    let mut params = Vec::new();
    for child in symbol.metadata().children() {
        if child.metadata().kind() == KestrelSymbolKind::Field {
            let name = child.metadata().name().value.clone();
            if let Some(typed) = child.metadata().get_behavior::<TypedBehavior>() {
                params.push(ParameterInfo {
                    label: format!("{}: {}", name, typed.ty()),
                    documentation: None,
                });
            }
        }
    }
    params
}

/// Placeholder for return type display when we don't have symbol info.
fn expr_type_display(_arguments: &[CallArgument]) -> &'static str {
    "?"
}

/// Text-scanning fallback for signature help when semantic model isn't available.
fn find_call_site_from_text(
    compilation: &Compilation,
    source: &str,
    offset: usize,
) -> Option<CallSiteInfo> {
    let before_cursor = &source[..offset.min(source.len())];

    let mut paren_depth = 0;
    let mut comma_count = 0;
    let mut open_paren_pos = None;

    for (i, ch) in before_cursor.char_indices().rev() {
        match ch {
            ')' => paren_depth += 1,
            '(' => {
                if paren_depth == 0 {
                    open_paren_pos = Some(i);
                    break;
                }
                paren_depth -= 1;
            },
            ',' if paren_depth == 0 => comma_count += 1,
            _ => {},
        }
    }

    let open_paren_pos = open_paren_pos?;

    let before_paren = &source[..open_paren_pos];
    let function_name = extract_identifier_before(before_paren)?;

    let model = compilation.semantic_model()?;

    fn find_function_by_name(
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        name: &str,
    ) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
        let metadata = symbol.metadata();
        if metadata.name().value == name && metadata.get_behavior::<CallableBehavior>().is_some() {
            return Some(symbol.clone());
        }
        for child in metadata.children() {
            if let Some(found) = find_function_by_name(&child, name) {
                return Some(found);
            }
        }
        None
    }

    let root = model.root();
    let mut function_symbol = None;
    for child in root.metadata().children() {
        if let Some(found) = find_function_by_name(&child, &function_name) {
            function_symbol = Some(found);
            break;
        }
    }

    let function_symbol = function_symbol?;
    let callable = function_symbol
        .metadata()
        .get_behavior::<CallableBehavior>()?;
    let parameters = build_parameter_list(&callable);

    Some(CallSiteInfo {
        function_name,
        parameters,
        return_type: callable.return_type().to_string(),
        active_parameter: comma_count,
    })
}

/// Extract an identifier from the end of a string.
fn extract_identifier_before(s: &str) -> Option<String> {
    let trimmed = s.trim_end();
    if trimmed.is_empty() {
        return None;
    }

    // Find the start of the identifier (scan backwards for non-identifier chars)
    let mut start = trimmed.len();
    for (i, ch) in trimmed.char_indices().rev() {
        if ch.is_alphanumeric() || ch == '_' {
            start = i;
        } else {
            break;
        }
    }

    let ident = &trimmed[start..];
    if ident.is_empty() || ident.chars().next()?.is_numeric() {
        return None;
    }

    Some(ident.to_string())
}

/// Find all symbol definitions in a file and return their positions.
///
/// This is useful for document symbols.
pub fn find_all_symbols_in_file(compilation: &Compilation, file_id: usize) -> Vec<SymbolInfo> {
    let mut symbols = Vec::new();

    let Some(model) = compilation.semantic_model() else {
        return symbols;
    };

    fn collect_symbols(
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        file_id: usize,
        symbols: &mut Vec<SymbolInfo>,
    ) {
        let metadata = symbol.metadata();
        let span = metadata.span();

        // Only include symbols defined in this file
        if span.file_id == file_id && !span.is_synthetic() {
            let kind = format!("{:?}", metadata.kind());
            let signature = build_signature(symbol);
            let decl_span = metadata.declaration_span();

            symbols.push(SymbolInfo {
                name: metadata.name().value.clone(),
                kind,
                signature,
                definition: Some((decl_span.file_id, decl_span.start, decl_span.end)),
            });
        }

        // Collect children
        for child in metadata.children() {
            collect_symbols(&child, file_id, symbols);
        }
    }

    let root = model.root();
    for child in root.metadata().children() {
        collect_symbols(&child, file_id, &mut symbols);
    }

    symbols
}

/// A completion item for LSP.
#[derive(Debug, Clone)]
pub struct CompletionItem {
    /// The label to display
    pub label: String,
    /// The kind of completion (field, method, etc.)
    pub kind: CompletionKind,
    /// Detail/signature information
    pub detail: Option<String>,
    /// Text to insert (if different from label)
    pub insert_text: Option<String>,
}

/// Kind of completion item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionKind {
    Field,
    Method,
    Function,
    Property,
}

/// Find completions for dot completion (e.g., `foo.` shows members of foo's type).
pub fn find_dot_completions(
    compilation: &Compilation,
    source: &str,
    file_id: usize,
    offset: usize,
) -> Vec<CompletionItem> {
    let mut completions = Vec::new();

    let model = match compilation.semantic_model() {
        Some(m) => m,
        None => return completions,
    };

    // Find the identifier before the dot
    let before_cursor = &source[..offset.min(source.len())];
    let trimmed = before_cursor.trim_end_matches('.');
    let var_name = match extract_identifier_before(trimmed) {
        Some(name) => name,
        None => return completions,
    };

    // Find the variable in the semantic model to get its type
    // We need to search for a local variable or parameter with this name
    let var_type = find_variable_type(compilation, file_id, offset, &var_name);

    let Some(ty) = var_type else {
        return completions;
    };

    // Based on the type, find available members
    match ty.kind() {
        TyKind::Struct { symbol, .. } => {
            // Add fields
            for child in symbol.metadata().children() {
                let kind = child.metadata().kind();
                if kind == KestrelSymbolKind::Field {
                    let name = child.metadata().name().value.clone();
                    let detail = child
                        .metadata()
                        .get_behavior::<TypedBehavior>()
                        .map(|t| t.ty().to_string());
                    completions.push(CompletionItem {
                        label: name,
                        kind: CompletionKind::Field,
                        detail,
                        insert_text: None,
                    });
                } else if kind == KestrelSymbolKind::Function {
                    let name = child.metadata().name().value.clone();
                    let detail = child
                        .metadata()
                        .get_behavior::<CallableBehavior>()
                        .map(|c| {
                            format!(
                                "({}) -> {}",
                                c.parameters()
                                    .iter()
                                    .map(|p| p.ty.to_string())
                                    .collect::<Vec<_>>()
                                    .join(", "),
                                c.return_type()
                            )
                        });
                    completions.push(CompletionItem {
                        label: name.clone(),
                        kind: CompletionKind::Method,
                        detail,
                        insert_text: Some(format!("{}()", name)),
                    });
                }
            }

            // Add extension methods
            let extensions = model
                .extension_registry()
                .get_extensions_for(symbol.metadata().id());
            for ext in extensions {
                for child in ext.metadata().children() {
                    if child.metadata().kind() == KestrelSymbolKind::Function {
                        let name = child.metadata().name().value.clone();
                        let detail = child
                            .metadata()
                            .get_behavior::<CallableBehavior>()
                            .map(|c| {
                                format!(
                                    "({}) -> {}",
                                    c.parameters()
                                        .iter()
                                        .map(|p| p.ty.to_string())
                                        .collect::<Vec<_>>()
                                        .join(", "),
                                    c.return_type()
                                )
                            });
                        completions.push(CompletionItem {
                            label: name.clone(),
                            kind: CompletionKind::Method,
                            detail,
                            insert_text: Some(format!("{}()", name)),
                        });
                    }
                }
            }
        },
        TyKind::Enum { symbol, .. } => {
            // Add enum methods
            for child in symbol.metadata().children() {
                if child.metadata().kind() == KestrelSymbolKind::Function {
                    let name = child.metadata().name().value.clone();
                    let detail = child
                        .metadata()
                        .get_behavior::<CallableBehavior>()
                        .map(|c| {
                            format!(
                                "({}) -> {}",
                                c.parameters()
                                    .iter()
                                    .map(|p| p.ty.to_string())
                                    .collect::<Vec<_>>()
                                    .join(", "),
                                c.return_type()
                            )
                        });
                    completions.push(CompletionItem {
                        label: name.clone(),
                        kind: CompletionKind::Method,
                        detail,
                        insert_text: Some(format!("{}()", name)),
                    });
                }
            }
        },
        TyKind::String => {
            // String is a builtin - we'd need to look up String extension methods
            // For now, just indicate it's a String
        },
        _ => {},
    }

    completions
}

/// Find the type of a variable at a given position.
fn find_variable_type(
    compilation: &Compilation,
    file_id: usize,
    offset: usize,
    var_name: &str,
) -> Option<kestrel_semantic_tree::ty::Ty> {
    let model = compilation.semantic_model()?;

    // Walk the symbol tree to find a symbol matching this name
    // that we're "inside" based on position
    fn find_in_symbol(
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        file_id: usize,
        offset: usize,
        var_name: &str,
    ) -> Option<kestrel_semantic_tree::ty::Ty> {
        let metadata = symbol.metadata();
        let span = metadata.span();

        // Check if we're inside this symbol's scope
        if span.file_id == file_id && offset >= span.start && offset <= span.end {
            // Check children for any symbol with this name that has a type
            for child in metadata.children() {
                let child_meta = child.metadata();

                // Check if this child has the name we're looking for
                if child_meta.name().value == var_name {
                    // Get its type from behaviors
                    if let Some(valued) = child_meta.get_behavior::<ValueBehavior>() {
                        return Some(valued.ty().clone());
                    }
                    if let Some(typed) = child_meta.get_behavior::<TypedBehavior>() {
                        return Some(typed.ty().clone());
                    }
                }

                // Recursively check children (for nested scopes)
                if let Some(ty) = find_in_symbol(&child, file_id, offset, var_name) {
                    return Some(ty);
                }
            }
        }

        None
    }

    let root = model.root();
    for child in root.metadata().children() {
        if let Some(ty) = find_in_symbol(&child, file_id, offset, var_name) {
            return Some(ty);
        }
    }

    None
}
