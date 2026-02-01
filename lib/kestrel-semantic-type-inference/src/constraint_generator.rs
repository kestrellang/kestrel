//! Constraint generation from code blocks.
//!
//! This module walks a `CodeBlock` and generates type inference constraints
//! for the expressions and statements within it.

use kestrel_semantic_tree::behavior::executable::CodeBlock;
use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_semantic_tree::expr::{ExprKind, Expression};
use kestrel_semantic_tree::pattern::{Pattern, PatternKind};
use kestrel_semantic_tree::stmt::{Statement, StatementKind};
use kestrel_semantic_tree::symbol::field::FieldSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::{Ty, TyKind};
use semantic_tree::symbol::Symbol;

use crate::constraint::ProtocolRef;
use crate::context::InferenceContext;

/// Generate type inference constraints for a code block.
///
/// This walks all statements and expressions in the block, generating
/// constraints that the solver will use to infer types.
///
/// # Arguments
/// * `ctx` - The inference context to add constraints to
/// * `block` - The code block to generate constraints for
/// * `return_type` - The expected return type (for yield expression)
pub fn generate_constraints(
    ctx: &mut InferenceContext<'_>,
    block: &CodeBlock,
    return_type: Option<&Ty>,
) {
    // Store return type in context for use by nested return statements
    ctx.set_return_type(return_type.cloned());

    // Process all statements
    for stmt in &block.statements {
        generate_statement_constraints(ctx, stmt);
    }

    // Process yield expression if present
    if let Some(yield_expr) = block.yield_expr() {
        generate_expression_constraints(ctx, yield_expr);

        // Constrain yield expression type to be promotable to return type
        // (allows implicit promotion from T to Optional[T] or Result[T, E])
        if let Some(ret_ty) = return_type {
            ctx.register_type(ret_ty);
            ctx.register_type(&yield_expr.ty);
            ctx.promotable(yield_expr.ty.id(), ret_ty.id(), yield_expr.id, yield_expr.span.clone());
        }
    }
}

/// Generate type inference constraints for default value expressions.
///
/// For each default value, generates constraints and ensures the expression
/// type is promotable to the parameter type.
///
/// # Arguments
/// * `ctx` - The inference context to add constraints to
/// * `default_values` - The default value expressions (None if parameter has no default)
/// * `param_types` - The expected parameter types, in the same order
pub fn generate_default_value_constraints(
    ctx: &mut InferenceContext<'_>,
    default_values: &[Option<Expression>],
    param_types: &[Ty],
) {
    for (default_opt, param_ty) in default_values.iter().zip(param_types.iter()) {
        if let Some(default_expr) = default_opt {
            // Generate constraints for the default expression
            generate_expression_constraints(ctx, default_expr);

            // Constrain the default expression type to be promotable to parameter type
            ctx.register_type(param_ty);
            ctx.register_type(&default_expr.ty);
            ctx.promotable(
                default_expr.ty.id(),
                param_ty.id(),
                default_expr.id,
                default_expr.span.clone(),
            );
        }
    }
}

/// Generate constraints for a statement.
fn generate_statement_constraints(ctx: &mut InferenceContext<'_>, stmt: &Statement) {
    match &stmt.kind {
        StatementKind::Binding { pattern, value } => {
            // Generate constraints for the pattern
            generate_pattern_constraints(ctx, pattern);

            // If there's an initializer, constrain it to be promotable to the pattern type
            // (allows implicit promotion from T to Optional[T] or Result[T, E])
            if let Some(init) = value {
                generate_expression_constraints(ctx, init);
                ctx.register_type(&init.ty);
                ctx.promotable(init.ty.id(), pattern.ty.id(), init.id, stmt.span.clone());
            }
        },
        StatementKind::Expr(expr) => {
            generate_expression_constraints(ctx, expr);
        },
        StatementKind::GuardLet {
            conditions,
            else_block,
        } => {
            // Generate constraints for each condition in the chain
            for condition in conditions {
                generate_if_condition_constraints(ctx, condition);
            }

            // Generate constraints for the else block statements
            for else_stmt in &else_block.statements {
                generate_statement_constraints(ctx, else_stmt);
            }
            // The else block's yield expression (if any)
            if let Some(yield_expr) = else_block.yield_expr() {
                generate_expression_constraints(ctx, yield_expr);
            }
        },
        StatementKind::Deinit { .. } => {
            // Deinit statement doesn't generate any type constraints
            // The move tracking is already handled during body resolution
        },
    }
}

/// Generate constraints for a pattern.
///
/// This registers the pattern's type and generates constraints based on the pattern kind:
/// - For tuples: creates tuple type constraints from elements
/// - For literals: constrains to the literal's type
/// - For enum variants: the case resolution happens later during type application
///
/// # Arguments
/// * `ctx` - The inference context to add constraints to
/// * `pattern` - The pattern to generate constraints for
pub fn generate_pattern_constraints(ctx: &mut InferenceContext<'_>, pattern: &Pattern) {
    // Register the pattern's type
    ctx.register_type(&pattern.ty);

    match &pattern.kind {
        PatternKind::Local { .. } => {
            // Local bindings just register their type - nothing more needed
        },

        PatternKind::Wildcard => {
            // Wildcard patterns match anything - type is already registered
        },

        PatternKind::Tuple {
            prefix,
            has_rest: _,
            suffix,
        } => {
            // For tuple patterns, generate constraints for each element
            // and ensure the tuple type matches
            if let TyKind::Tuple(elem_tys) = pattern.ty.kind() {
                // Handle prefix elements (matched from the start)
                for (i, elem) in prefix.iter().enumerate() {
                    generate_pattern_constraints(ctx, elem);
                    if let Some(elem_ty) = elem_tys.get(i) {
                        ctx.register_type(elem_ty);
                        ctx.equate(elem.ty.id(), elem_ty.id(), elem.span.clone());
                    }
                }

                // Handle suffix elements (matched from the end)
                let suffix_start = elem_tys.len().saturating_sub(suffix.len());
                for (i, elem) in suffix.iter().enumerate() {
                    generate_pattern_constraints(ctx, elem);
                    if let Some(elem_ty) = elem_tys.get(suffix_start + i) {
                        ctx.register_type(elem_ty);
                        ctx.equate(elem.ty.id(), elem_ty.id(), elem.span.clone());
                    }
                }
            } else {
                // Pattern type is not a tuple - still process all elements
                for elem in prefix.iter().chain(suffix.iter()) {
                    generate_pattern_constraints(ctx, elem);
                }
            }
        },

        PatternKind::Literal { value } => {
            // For literal patterns with inference placeholders, add ExpressibleBy* constraints
            // so the solver can unify with the scrutinee type.
            //
            // When the protocol is available: add a conformance constraint so the solver
            // checks that the scrutinee type conforms to ExpressibleByIntLiteral (etc).
            //
            // When the protocol is NOT available (e.g., tests without prelude): don't add
            // any constraint here. The match expression's equate constraint will unify
            // the pattern's infer type directly with the scrutinee type, allowing primitive
            // types like lang.i32 to match integer literal patterns.
            use kestrel_semantic_tree::builtins::LanguageFeature;
            use kestrel_semantic_tree::expr::LiteralValue;

            let feature = match value {
                LiteralValue::Integer(_) => Some(LanguageFeature::ExpressibleByIntLiteral),
                LiteralValue::Float(_) => Some(LanguageFeature::ExpressibleByFloatLiteral),
                LiteralValue::String(_) => Some(LanguageFeature::ExpressibleByStringLiteral),
                LiteralValue::Char(_) => Some(LanguageFeature::ExpressibleByCharLiteral),
                LiteralValue::Bool(_) => Some(LanguageFeature::ExpressibleByBoolLiteral),
                LiteralValue::Null => Some(LanguageFeature::ExpressibleByNullLiteral),
                LiteralValue::Unit => None,
            };

            if let Some(feature) = feature
                && let Some(protocol_id) = ctx.oracle().builtin_protocol(feature)
            {
                // Protocol is registered - add conformance constraint
                let protocol_ref = ProtocolRef::new(protocol_id, pattern.span.clone());
                ctx.conforms(pattern.ty.id(), protocol_ref);
            }
            // If protocol not registered, don't add any constraint - the match
            // expression's equate constraint will handle unification directly
        },

        PatternKind::EnumVariant {
            case_name,
            bindings,
            ..
        } => {
            // For enum patterns, generate constraints for each binding
            for binding in bindings {
                generate_pattern_constraints(ctx, &binding.pattern);
            }

            // Generate a constraint to validate the enum case exists and connect
            // binding types to the enum case's parameter types.
            let binding_tys: Vec<(Option<String>, _)> = bindings
                .iter()
                .map(|b| {
                    ctx.register_type(&b.pattern.ty);
                    (b.label.clone(), b.pattern.ty.id())
                })
                .collect();

            ctx.enum_pattern_binding(
                pattern.ty.id(),
                case_name.clone(),
                binding_tys,
                pattern.span.clone(),
            );
        },

        PatternKind::Range { .. } => {
            // Range patterns have concrete types (Int or Char) - type is already set
        },

        PatternKind::Struct {
            struct_name,
            fields,
            has_rest,
            ..
        } => {
            // For struct patterns, generate constraints for each field pattern
            for field in fields {
                generate_pattern_constraints(ctx, &field.pattern);
            }

            // Generate struct pattern binding constraint to connect field types
            // to the struct's field types
            let field_bindings: Vec<(String, _)> = fields
                .iter()
                .map(|f| {
                    ctx.register_type(&f.pattern.ty);
                    (f.field_name.clone(), f.pattern.ty.id())
                })
                .collect();

            ctx.struct_pattern_binding(
                pattern.ty.id(),
                struct_name.clone(),
                field_bindings,
                *has_rest,
                pattern.span.clone(),
            );
        },

        PatternKind::Array { prefix, suffix, .. } => {
            // For array patterns, generate constraints for prefix and suffix patterns
            for elem in prefix {
                generate_pattern_constraints(ctx, elem);
            }
            for elem in suffix {
                generate_pattern_constraints(ctx, elem);
            }
            // The rest pattern (.. or ..name) is just a marker/binding - no pattern constraints needed
        },

        PatternKind::Or { alternatives } => {
            // For or-patterns, generate constraints for each alternative
            // and equate their types with the or-pattern's type
            for alt in alternatives {
                generate_pattern_constraints(ctx, alt);
                // Each alternative's type must equal the or-pattern's type
                ctx.equate(pattern.ty.id(), alt.ty.id(), alt.span.clone());
            }
        },

        PatternKind::At { subpattern, .. } => {
            // For at-patterns, generate constraints for the subpattern
            generate_pattern_constraints(ctx, subpattern);
            // The @ pattern's type must equal the subpattern's type
            ctx.equate(pattern.ty.id(), subpattern.ty.id(), pattern.span.clone());
        },

        PatternKind::Rest => {
            // Rest patterns are just markers - no additional constraints needed
        },

        PatternKind::Error => {
            // Error patterns are poison values - don't generate constraints
        },
    }
}

/// Generate constraints for an if condition (used by if-let, while-let, guard-let chains).
fn generate_if_condition_constraints(
    ctx: &mut InferenceContext<'_>,
    condition: &kestrel_semantic_tree::expr::IfCondition,
) {
    use kestrel_semantic_tree::expr::IfCondition;
    match condition {
        IfCondition::Expr(expr) => {
            generate_expression_constraints(ctx, expr);
            // Note: We don't add a conformance constraint for BooleanConditional here
            // because primitive lang.bool doesn't implement protocols. Instead, the
            // type checker validates conditions in check_if_condition().
        },
        IfCondition::Let { pattern, value, .. } => {
            // Generate constraints for the scrutinee expression
            generate_expression_constraints(ctx, value);
            // Generate constraints for the pattern (pattern type == scrutinee type)
            generate_pattern_constraints(ctx, pattern);
            ctx.equate(pattern.ty.id(), value.ty.id(), value.span.clone());
        },
    }
}

/// Generate constraints for an expression.
fn generate_expression_constraints(ctx: &mut InferenceContext<'_>, expr: &Expression) {
    // Register this expression's type
    ctx.register_type(&expr.ty);

    match &expr.kind {
        // Literals: generate ExpressibleBy* protocol constraints or use default types
        ExprKind::Literal(lit_val) => {
            use kestrel_semantic_tree::builtins::LanguageFeature;
            use kestrel_semantic_tree::expr::LiteralValue;
            use kestrel_semantic_tree::ty::{FloatBits, IntBits, Ty};

            let (feature, default_ty) = match lit_val {
                LiteralValue::Integer(_) => (
                    Some(LanguageFeature::ExpressibleByIntLiteral),
                    Some(Ty::int(IntBits::I64, expr.span.clone())),
                ),
                LiteralValue::Float(_) => (
                    Some(LanguageFeature::ExpressibleByFloatLiteral),
                    Some(Ty::float(FloatBits::F64, expr.span.clone())),
                ),
                LiteralValue::String(_) => (
                    Some(LanguageFeature::ExpressibleByStringLiteral),
                    Some(Ty::string(expr.span.clone())),
                ),
                LiteralValue::Char(_) => (
                    Some(LanguageFeature::ExpressibleByCharLiteral),
                    Some(Ty::int(IntBits::I32, expr.span.clone())),
                ),
                LiteralValue::Bool(_) => (
                    Some(LanguageFeature::ExpressibleByBoolLiteral),
                    Some(Ty::bool(expr.span.clone())),
                ),
                LiteralValue::Null => (
                    Some(LanguageFeature::ExpressibleByNullLiteral),
                    None, // Default handled specially in solver (generic NullLiteralType[T])
                ),
                LiteralValue::Unit => (None, None), // Unit literal has concrete type
            };

            if let Some(feature) = feature {
                if let Some(protocol_id) = ctx.oracle().builtin_protocol(feature) {
                    // Protocol is registered - use protocol-based inference
                    let protocol_ref = ProtocolRef::new(protocol_id, expr.span.clone());
                    ctx.conforms(expr.ty.id(), protocol_ref);
                } else if let Some(default_ty) = default_ty {
                    // Protocol not registered (e.g., in tests) - use default type directly
                    ctx.register_type(&default_ty);
                    ctx.equate(expr.ty.id(), default_ty.id(), expr.span.clone());
                }
            }
        },

        // Arrays: type conforms to _ExpressibleByArrayLiteral, elements have Element type
        ExprKind::Array(elements) => {
            use kestrel_semantic_tree::builtins::LanguageFeature;

            // Add conformance constraint to _ExpressibleByArrayLiteral protocol
            // This allows array literals to be assigned to custom types like Style
            if let Some(protocol_id) = ctx
                .oracle()
                .builtin_protocol(LanguageFeature::_ExpressibleByArrayLiteral)
            {
                let protocol_ref = ProtocolRef::new(protocol_id, expr.span.clone());
                ctx.conforms(expr.ty.id(), protocol_ref);
            }

            // Create an infer type for the element type
            // This will be linked to the array type's Element associated type
            let elem_ty = Ty::infer(expr.span.clone());
            ctx.register_type(&elem_ty);

            // Add normalizes constraint: array_type.Element = elem_ty
            // This resolves the Element associated type from the expected type
            // (e.g., Style.Element = StyleOption)
            ctx.normalizes(
                expr.ty.id(),
                "Element".to_string(),
                elem_ty.id(),
                expr.span.clone(),
            );

            // All elements must have the same type (the Element type)
            for elem in elements {
                generate_expression_constraints(ctx, elem);
                ctx.equate(elem.ty.id(), elem_ty.id(), elem.span.clone());
            }
        },

        // Dictionaries: type conforms to _ExpressibleByDictionaryLiteral, pairs have Key/Value types
        ExprKind::Dictionary(pairs) => {
            use kestrel_semantic_tree::builtins::LanguageFeature;

            // Add conformance constraint to _ExpressibleByDictionaryLiteral protocol
            if let Some(protocol_id) = ctx
                .oracle()
                .builtin_protocol(LanguageFeature::_ExpressibleByDictionaryLiteral)
            {
                let protocol_ref = ProtocolRef::new(protocol_id, expr.span.clone());
                ctx.conforms(expr.ty.id(), protocol_ref);
            }

            // Create infer types for Key and Value
            let key_ty = Ty::infer(expr.span.clone());
            let value_ty = Ty::infer(expr.span.clone());
            ctx.register_type(&key_ty);
            ctx.register_type(&value_ty);

            // Add normalizes constraints for Key and Value associated types
            ctx.normalizes(
                expr.ty.id(),
                "Key".to_string(),
                key_ty.id(),
                expr.span.clone(),
            );
            ctx.normalizes(
                expr.ty.id(),
                "Value".to_string(),
                value_ty.id(),
                expr.span.clone(),
            );

            // All keys must unify to Key type, all values to Value type
            for (key, value) in pairs {
                generate_expression_constraints(ctx, key);
                generate_expression_constraints(ctx, value);
                ctx.equate(key.ty.id(), key_ty.id(), key.span.clone());
                ctx.equate(value.ty.id(), value_ty.id(), value.span.clone());
            }
        },

        // Tuples: each element has its corresponding type
        ExprKind::Tuple(elements) => {
            if let TyKind::Tuple(elem_tys) = expr.ty.kind() {
                for (elem, elem_ty) in elements.iter().zip(elem_tys.iter()) {
                    generate_expression_constraints(ctx, elem);
                    ctx.register_type(elem_ty);
                    ctx.equate(elem.ty.id(), elem_ty.id(), elem.span.clone());
                }
            }
        },

        // Grouping: just process the inner expression
        ExprKind::Grouping(inner) => {
            generate_expression_constraints(ctx, inner);
            ctx.equate(inner.ty.id(), expr.ty.id(), expr.span.clone());
        },

        // References: type is already set during binding
        ExprKind::LocalRef(_) | ExprKind::SymbolRef(_) | ExprKind::TypeRef(_) => {},
        ExprKind::OverloadedRef(_)
        | ExprKind::TypeParameterRef(_)
        | ExprKind::AssociatedTypeRef => {},

        // Field access: type is the field type
        ExprKind::FieldAccess { object, field } => {
            generate_expression_constraints(ctx, object);
            // If the expression type is Infer, generate a member access constraint
            // so the solver can resolve the field type once the receiver type is known
            if matches!(expr.ty.kind(), TyKind::Infer) {
                ctx.register_type(&object.ty);
                ctx.register_type(&expr.ty);
                ctx.member_access(
                    object.ty.id(),
                    field.clone(),
                    false,  // instance access
                    vec![], // no arguments for field access
                    expr.ty.id(),
                    expr.id,
                    expr.span.clone(),
                );
            }
        },

        // Tuple index: type is the element type
        ExprKind::TupleIndex { tuple, .. } => {
            generate_expression_constraints(ctx, tuple);
        },

        // Method reference: process receiver and check for protocol conformance
        ExprKind::MethodRef {
            receiver,
            candidates,
            ..
        } => {
            generate_expression_constraints(ctx, receiver);

            // If any candidate is a protocol method, add a conformance constraint.
            // Skip for type parameters - their conformance is verified at bind time,
            // including local where clause constraints that we can't see here.
            if !matches!(receiver.ty.kind(), TyKind::TypeParameter(_)) {
                for candidate_id in candidates {
                    if let Some(protocol_id) = ctx.oracle().protocol_for_method(*candidate_id) {
                        let protocol_ref = ProtocolRef::new(protocol_id, expr.span.clone());
                        ctx.conforms(receiver.ty.id(), protocol_ref);
                    }
                }
            }
        },

        // Primitive method reference: this should only appear if the primitive method
        // was NOT called. Emit an error because primitive methods can't be first-class values.
        ExprKind::PrimitiveMethodRef { receiver, method } => {
            generate_expression_constraints(ctx, receiver);
            // Emit error - primitive methods must be called
            ctx.add_error(crate::InferenceError::primitive_method_not_called(
                method.name().to_string(),
                receiver.ty.to_string(),
                expr.span.clone(),
            ));
        },

        // Calls
        ExprKind::Call {
            callee, arguments, ..
        } => {
            generate_expression_constraints(ctx, callee);
            for arg in arguments {
                generate_expression_constraints(ctx, &arg.value);
            }

            // Generate constraints between argument types and parameter types
            // This enables bidirectional type inference for closures passed as arguments
            // and allows implicit promotion from T to Optional[T] or Result[T, E]
            match callee.ty.kind() {
                TyKind::Function { params, .. } => {
                    for (arg, param_ty) in arguments.iter().zip(params.iter()) {
                        ctx.register_type(&arg.value.ty);
                        ctx.register_type(param_ty);
                        ctx.promotable(arg.value.ty.id(), param_ty.id(), arg.value.id, arg.span.clone());
                    }
                },
                TyKind::UnresolvedFunction { .. } => {
                    // For unresolved functions, create a concrete function type from
                    // the call site and equate it with the callee's type. This allows
                    // the solver to unify the UnresolvedFunction with the expected
                    // function signature based on how it's being called.
                    let arg_types: Vec<Ty> =
                        arguments.iter().map(|arg| arg.value.ty.clone()).collect();
                    let expected_fn_ty =
                        Ty::function(arg_types, expr.ty.clone(), expr.span.clone());
                    ctx.register_type(&expected_fn_ty);
                    ctx.register_type(&callee.ty);
                    ctx.equate(callee.ty.id(), expected_fn_ty.id(), expr.span.clone());
                },
                _ => {
                    // Check if callee is a MethodRef - if so, generate member_access constraint
                    // for method resolution. This enables the MethodRef pattern for protocol
                    // method calls (used by desugared for-loops for iter()/next()).
                    if let ExprKind::MethodRef {
                        receiver,
                        method_name,
                        ..
                    } = &callee.kind
                    {
                        let arg_ty_ids: Vec<_> =
                            arguments.iter().map(|a| a.value.ty.id()).collect();
                        ctx.register_type(&receiver.ty);
                        ctx.register_type(&expr.ty);
                        ctx.member_access(
                            receiver.ty.id(),
                            method_name.clone(),
                            false,      // instance method call
                            arg_ty_ids, // argument types for parameter constraint generation
                            expr.ty.id(),
                            expr.id,
                            expr.span.clone(),
                        );

                        // For zero-argument methods on literals, speculatively equate receiver with result.
                        // This enables bidirectional type inference for Self-returning operators like
                        // negate(), bitwiseNot(), etc. When the expected result type is known (e.g., Int16),
                        // this constraint propagates that type back to the receiver (the literal),
                        // preventing default literal inference.
                        if arguments.is_empty() && matches!(receiver.kind, ExprKind::Literal(_)) {
                            ctx.equate(receiver.ty.id(), expr.ty.id(), expr.span.clone());
                        }
                    }
                    // Otherwise callee type is not a function - might be an error or inference needed
                },
            }
        },

        ExprKind::PrimitiveMethodCall {
            receiver,
            arguments,
            ..
        } => {
            generate_expression_constraints(ctx, receiver);
            for arg in arguments {
                generate_expression_constraints(ctx, &arg.value);
            }
        },

        ExprKind::DeferredMethodCall {
            receiver,
            method_name,
            arguments,
        } => {
            generate_expression_constraints(ctx, receiver);
            for arg in arguments {
                generate_expression_constraints(ctx, &arg.value);
            }
            // Collect argument type IDs for constraint generation
            // When the method is resolved, these will be constrained to match parameter types
            let arg_ty_ids: Vec<_> = arguments.iter().map(|a| a.value.ty.id()).collect();

            // Generate a member access constraint to resolve the method once receiver type is known
            ctx.register_type(&receiver.ty);
            ctx.register_type(&expr.ty);
            ctx.member_access(
                receiver.ty.id(),
                method_name.clone(),
                false,      // instance method call
                arg_ty_ids, // argument types for parameter constraint generation
                expr.ty.id(),
                expr.id,
                expr.span.clone(),
            );

            // NOTE: We previously had a speculative equate here for zero-argument methods on
            // literals that would equate receiver with result. This enabled bidirectional type
            // inference for Self-returning operators like negate(). However, this caused issues
            // for methods that return a different type (like String.toCString() -> CString):
            // the speculative equate would set the result type to String before the method
            // could be resolved, then conflict with the actual return type CString.
            //
            // For now, we disable this optimization. Self-returning operator type inference
            // may need a different approach (e.g., checking if the resolved method actually
            // returns Self before applying the equate constraint).
        },

        ExprKind::DeferredStaticCall {
            target_ty,
            method_name,
            arguments,
            protocol_candidates,
        } => {
            // Generate constraints for arguments
            for arg in arguments {
                generate_expression_constraints(ctx, &arg.value);
            }

            // Collect argument type IDs for constraint generation
            let arg_ty_ids: Vec<_> = arguments.iter().map(|a| a.value.ty.id()).collect();

            // Register target type and result type
            ctx.register_type(target_ty);
            ctx.register_type(&expr.ty);

            // If protocol candidates are provided, emit conformance constraints.
            // This produces better error messages ("does not conform to X" instead of "no member Y").
            for candidate_id in protocol_candidates {
                if let Some(protocol_id) = ctx.oracle().protocol_for_method(*candidate_id) {
                    let protocol_ref = ProtocolRef::new(protocol_id, expr.span.clone());
                    ctx.conforms(target_ty.id(), protocol_ref);
                }
            }

            // Generate a member access constraint for the static method
            // is_static = true indicates this is a static method lookup
            ctx.member_access(
                target_ty.id(),
                method_name.clone(),
                true, // is_static = true for static method call
                arg_ty_ids,
                expr.ty.id(),
                expr.id,
                expr.span.clone(),
            );
        },

        ExprKind::ImplicitStructInit { arguments, .. } => {
            for arg in arguments {
                generate_expression_constraints(ctx, &arg.value);
            }

            // Generate constraints between argument types and field types
            // This enables bidirectional type inference for closures in struct fields
            if let Some((struct_sym, substitutions)) = expr.ty.as_struct_with_subs() {
                // Get field symbols from struct children
                let fields: Vec<_> = struct_sym
                    .metadata()
                    .children()
                    .into_iter()
                    .filter(|c| c.metadata().kind() == KestrelSymbolKind::Field)
                    .filter_map(|c| c.downcast_arc::<FieldSymbol>().ok())
                    .collect();

                // Constrain each argument type to be promotable to its corresponding field type
                // (allows implicit promotion from T to Optional[T] or Result[T, E])
                for (arg, field) in arguments.iter().zip(fields.iter()) {
                    // Get field type from TypedBehavior (resolved type) or fallback to field_type
                    let raw_field_ty = field
                        .metadata()
                        .get_behavior::<TypedBehavior>()
                        .map(|typed| typed.ty().clone())
                        .unwrap_or_else(|| field.field_type().clone());
                    let field_ty = raw_field_ty.apply_substitutions(substitutions);
                    ctx.register_type(&arg.value.ty);
                    ctx.register_type(&field_ty);
                    ctx.promotable(arg.value.ty.id(), field_ty.id(), arg.value.id, arg.span.clone());
                }
            }
        },

        // Delegating init - just generate constraints for arguments
        ExprKind::DelegatingInit { arguments, .. } => {
            for arg in arguments {
                generate_expression_constraints(ctx, &arg.value);
            }
        },

        // Assignment
        // (allows implicit promotion from T to Optional[T] or Result[T, E])
        ExprKind::Assignment { target, value } => {
            generate_expression_constraints(ctx, target);
            generate_expression_constraints(ctx, value);
            ctx.promotable(value.ty.id(), target.ty.id(), value.id, expr.span.clone());
        },

        // Control flow
        ExprKind::If {
            conditions,
            then_branch,
            then_value,
            else_branch,
        } => {
            // Process each condition
            for condition in conditions {
                generate_if_condition_constraints(ctx, condition);
            }

            // Process then branch
            for stmt in then_branch {
                generate_statement_constraints(ctx, stmt);
            }
            if let Some(then_val) = then_value {
                generate_expression_constraints(ctx, then_val);
                // Only constrain then value type with expression type when there's an else branch.
                // Without else, the if expression type is () and the then value is discarded.
                // (allows implicit promotion from T to Optional[T] or Result[T, E])
                if else_branch.is_some() {
                    ctx.promotable(then_val.ty.id(), expr.ty.id(), then_val.id, then_val.span.clone());
                }
            }

            // Process else branch
            if let Some(else_br) = else_branch {
                match else_br {
                    kestrel_semantic_tree::expr::ElseBranch::Block { statements, value } => {
                        for stmt in statements {
                            generate_statement_constraints(ctx, stmt);
                        }
                        if let Some(else_val) = value {
                            generate_expression_constraints(ctx, else_val);
                            // Else branch value type must be promotable to expression type
                            // (allows implicit promotion from T to Optional[T] or Result[T, E])
                            ctx.promotable(else_val.ty.id(), expr.ty.id(), else_val.id, else_val.span.clone());
                        }
                    },
                    kestrel_semantic_tree::expr::ElseBranch::ElseIf(else_if) => {
                        generate_expression_constraints(ctx, else_if);
                        // Else-if expression type must be promotable to expression type
                        // (allows implicit promotion from T to Optional[T] or Result[T, E])
                        ctx.promotable(else_if.ty.id(), expr.ty.id(), else_if.id, else_if.span.clone());
                    },
                }
            }
        },

        ExprKind::While {
            condition, body, ..
        } => {
            generate_expression_constraints(ctx, condition);
            // Note: We don't add a conformance constraint for BooleanConditional here
            // because primitive lang.bool doesn't implement protocols. Instead, the
            // type checker validates conditions in check_while_condition().

            for stmt in body {
                generate_statement_constraints(ctx, stmt);
            }
        },

        ExprKind::WhileLet {
            conditions, body, ..
        } => {
            // Generate constraints for each condition in the chain
            for condition in conditions {
                generate_if_condition_constraints(ctx, condition);
            }

            // Generate constraints for body statements
            for stmt in body {
                generate_statement_constraints(ctx, stmt);
            }
        },

        ExprKind::Loop { body, .. } => {
            for stmt in body {
                generate_statement_constraints(ctx, stmt);
            }
        },

        ExprKind::Break { .. } | ExprKind::Continue { .. } => {},

        ExprKind::Return { value } => {
            if let Some(val) = value {
                generate_expression_constraints(ctx, val);

                // Constrain return value type to be promotable to function return type
                // (allows implicit promotion from T to Optional[T] or Result[T, E])
                if let Some(ret_ty) = ctx.return_type().cloned() {
                    ctx.register_type(&ret_ty);
                    ctx.register_type(&val.ty);
                    ctx.promotable(val.ty.id(), ret_ty.id(), val.id, val.span.clone());
                }
            } else {
                // Return with no value - equate Unit with return type
                if let Some(ret_ty) = ctx.return_type().cloned() {
                    let unit_ty = Ty::unit(expr.span.clone());
                    ctx.register_type(&unit_ty);
                    ctx.register_type(&ret_ty);
                    ctx.equate(unit_ty.id(), ret_ty.id(), expr.span.clone());
                }
            }
        },

        ExprKind::Throw { value } => {
            // Throw desugars to return R.fromResidual(value)
            // We just need to generate constraints for the value expression
            // The type system will validate that the return type implements FromResidual
            generate_expression_constraints(ctx, value);
        },

        ExprKind::Closure {
            body,
            tail_expr,
            params,
            uses_it,
            implicit_param,
            ..
        } => {
            // Register the closure type
            ctx.register_type(&expr.ty);

            // Handle based on the closure's type
            match expr.ty.kind() {
                // Explicit params - closure has a concrete function type
                TyKind::Function {
                    params: closure_param_tys,
                    return_type: closure_return_ty,
                } => {
                    ctx.register_type(closure_return_ty);

                    // Record closure metadata for better error messages
                    let param_count = params.as_ref().map(|p| p.len()).unwrap_or(0);
                    let has_explicit_params = params.is_some();
                    ctx.register_closure_metadata(crate::context::ClosureMetadata {
                        expr_id: expr.id,
                        param_count,
                        uses_it: *uses_it,
                        has_explicit_params,
                        span: expr.span.clone(),
                        ty_id: expr.ty.id(),
                    });

                    // Register parameter types and create constraints
                    if let Some(param_list) = params {
                        for (param, param_ty) in param_list.iter().zip(closure_param_tys.iter()) {
                            ctx.register_type(&param.ty);
                            ctx.register_type(param_ty);
                            // Equate the parameter's declared/inferred type with the function type parameter
                            ctx.equate(param.ty.id(), param_ty.id(), param.span.clone());
                        }
                    }

                    // Handle implicit `it` parameter constraints
                    if let Some((_, it_ty, it_span)) = implicit_param
                        && let Some(first_param_ty) = closure_param_tys.first()
                    {
                        ctx.register_type(it_ty);
                        ctx.register_type(first_param_ty);
                        // Equate `it` type with the first function parameter type
                        ctx.equate(it_ty.id(), first_param_ty.id(), it_span.clone());
                    }

                    // Generate constraints for body statements
                    for stmt in body {
                        generate_statement_constraints(ctx, stmt);
                    }

                    // Generate constraints for tail expression and return type
                    if let Some(tail) = tail_expr {
                        generate_expression_constraints(ctx, tail);
                        // Equate tail expression type with closure return type
                        ctx.equate(tail.ty.id(), closure_return_ty.id(), tail.span.clone());
                    } else {
                        // No tail expression means return type should be Unit
                        let unit_ty = Ty::unit(expr.span.clone());
                        ctx.register_type(&unit_ty);
                        ctx.equate(unit_ty.id(), closure_return_ty.id(), expr.span.clone());
                    }
                },

                // UnresolvedFunction - closure without explicit params
                TyKind::UnresolvedFunction {
                    param_info,
                    return_type,
                } => {
                    use kestrel_semantic_tree::ty::ParamInfo;

                    // Register nested types
                    ctx.register_type(return_type);

                    // Register and handle param info
                    match param_info {
                        ParamInfo::ImplicitIt { it_type } => {
                            ctx.register_type(it_type);
                            // Record closure metadata for better error messages
                            ctx.register_closure_metadata(crate::context::ClosureMetadata {
                                expr_id: expr.id,
                                param_count: 1, // ImplicitIt means exactly 1 param
                                uses_it: true,
                                has_explicit_params: false,
                                span: expr.span.clone(),
                                ty_id: expr.ty.id(),
                            });
                        },
                        ParamInfo::Unconstrained => {
                            // Record closure metadata for better error messages
                            ctx.register_closure_metadata(crate::context::ClosureMetadata {
                                expr_id: expr.id,
                                param_count: 0, // Will be determined by context
                                uses_it: false,
                                has_explicit_params: false,
                                span: expr.span.clone(),
                                ty_id: expr.ty.id(),
                            });
                        },
                        ParamInfo::Explicit { param_types } => {
                            for pt in param_types {
                                ctx.register_type(pt);
                            }
                            // Record closure metadata for better error messages
                            ctx.register_closure_metadata(crate::context::ClosureMetadata {
                                expr_id: expr.id,
                                param_count: param_types.len(),
                                uses_it: false,
                                has_explicit_params: true,
                                span: expr.span.clone(),
                                ty_id: expr.ty.id(),
                            });
                        },
                    }

                    // Generate constraints for body statements
                    for stmt in body {
                        generate_statement_constraints(ctx, stmt);
                    }

                    // Equate tail expression with return type
                    if let Some(tail) = tail_expr {
                        generate_expression_constraints(ctx, tail);
                        ctx.equate(tail.ty.id(), return_type.id(), tail.span.clone());
                    } else {
                        // No tail expression means return type should be Unit
                        let unit_ty = Ty::unit(expr.span.clone());
                        ctx.register_type(&unit_ty);
                        ctx.equate(unit_ty.id(), return_type.id(), expr.span.clone());
                    }
                },

                // Fallback - shouldn't happen with well-formed trees
                _ => {
                    for stmt in body {
                        generate_statement_constraints(ctx, stmt);
                    }
                    if let Some(tail) = tail_expr {
                        generate_expression_constraints(ctx, tail);
                    }
                },
            }
        },

        // Enum case reference: type is already set during binding
        ExprKind::EnumCase { .. } => {},

        // Implicit member access: will be resolved during type inference
        ExprKind::ImplicitMemberAccess {
            member_name,
            arguments,
        } => {
            // Register the expression type
            ctx.register_type(&expr.ty);

            // Process argument expressions and collect their type IDs
            let argument_tys: Vec<(Option<String>, _)> = arguments
                .as_ref()
                .map(|args| {
                    args.iter()
                        .map(|arg| {
                            generate_expression_constraints(ctx, &arg.value);
                            ctx.register_type(&arg.value.ty);
                            (arg.label.clone(), arg.value.ty.id())
                        })
                        .collect()
                })
                .unwrap_or_default();

            // Generate the ImplicitMember constraint
            ctx.implicit_member(
                expr.ty.id(),
                member_name.clone(),
                argument_tys,
                expr.id,
                expr.span.clone(),
            );
        },

        ExprKind::Match { scrutinee, arms } => {
            // Generate constraints for the scrutinee
            generate_expression_constraints(ctx, scrutinee);
            ctx.register_type(&scrutinee.ty);

            // Generate constraints for each arm
            for arm in arms {
                // Generate constraints for the pattern
                generate_pattern_constraints(ctx, &arm.pattern);
                ctx.register_type(&arm.pattern.ty);

                // Pattern type must match scrutinee type
                ctx.equate(
                    arm.pattern.ty.id(),
                    scrutinee.ty.id(),
                    arm.pattern.span.clone(),
                );

                // Generate constraints for the guard if present
                if let Some(guard) = &arm.guard {
                    generate_expression_constraints(ctx, guard);
                    ctx.register_type(&guard.ty);
                    // Note: We don't add a conformance constraint for BooleanConditional here
                    // because primitive lang.bool doesn't implement protocols. The type checker
                    // will validate guard conditions separately.
                }

                // Generate constraints for the body
                generate_expression_constraints(ctx, &arm.body);
                ctx.register_type(&arm.body.ty);

                // Body type contributes to match expression type
                // All arms should have compatible types
                // (allows implicit promotion from T to Optional[T] or Result[T, E])
                ctx.promotable(arm.body.ty.id(), expr.ty.id(), arm.body.id, arm.body.span.clone());
            }
        },

        ExprKind::Block { statements, value } => {
            // Generate constraints for statements
            for stmt in statements {
                generate_statement_constraints(ctx, stmt);
            }
            // Generate constraints for the value expression if present
            if let Some(val) = value {
                generate_expression_constraints(ctx, val);
                ctx.register_type(&val.ty);
                // Block type equals value type
                ctx.equate(expr.ty.id(), val.ty.id(), val.span.clone());
            }
            // If no value, block type should be unit (already set in AST)
        },

        // Language intrinsics - process arguments and generate parameter constraints
        ExprKind::LangIntrinsic {
            intrinsic,
            arguments,
        } => {
            for arg in arguments {
                generate_expression_constraints(ctx, &arg.value);
                ctx.register_type(&arg.value.ty);
            }

            // Generate constraints between argument types and expected parameter types
            // This enables type inference to propagate from the intrinsic's signature to arguments
            use kestrel_semantic_tree::expr::LangIntrinsic;
            match intrinsic {
                // CastPtr expects lang.ptr[_] as input and returns lang.ptr[T]
                // If the argument's pointee type is unknown (Ty::infer), we can infer it from target_ty
                // This enables lang.cast_ptr[T](lang.ptr_null()) to work
                LangIntrinsic::CastPtr { target_ty } => {
                    if let Some(arg) = arguments.first() {
                        // The argument should be Pointer[SomeType]
                        // Extract the pointee type from the argument if it's a pointer
                        if let TyKind::Pointer(arg_pointee) = arg.value.ty.kind() {
                            // Register both types for inference
                            ctx.register_type(arg_pointee);
                            ctx.register_type(target_ty);

                            // If arg_pointee is infer (from ptr_null), equate with target_ty
                            // This allows ptr_null() to infer its type from the cast
                            if matches!(arg_pointee.kind(), TyKind::Infer) {
                                ctx.equate(arg_pointee.id(), target_ty.id(), arg.span.clone());
                            }
                        }
                    }
                },

                // Other pointer intrinsics with type parameters
                LangIntrinsic::PtrRead { pointee_ty } => {
                    if let Some(arg) = arguments.first() {
                        // Argument should be Pointer[T], equate with pointee_ty
                        if let TyKind::Pointer(arg_pointee) = arg.value.ty.kind() {
                            ctx.register_type(arg_pointee);
                            ctx.register_type(pointee_ty);
                            ctx.equate(arg_pointee.id(), pointee_ty.id(), arg.span.clone());
                        }
                    }
                },

                LangIntrinsic::PtrWrite { pointee_ty } => {
                    if let Some(ptr_arg) = arguments.first() {
                        // First argument should be Pointer[T]
                        if let TyKind::Pointer(arg_pointee) = ptr_arg.value.ty.kind() {
                            ctx.register_type(arg_pointee);
                            ctx.register_type(pointee_ty);
                            ctx.equate(arg_pointee.id(), pointee_ty.id(), ptr_arg.span.clone());
                        }
                    }
                    if let Some(value_arg) = arguments.get(1) {
                        // Second argument should be T
                        ctx.register_type(pointee_ty);
                        ctx.equate(
                            value_arg.value.ty.id(),
                            pointee_ty.id(),
                            value_arg.span.clone(),
                        );
                    }
                },

                LangIntrinsic::PtrTo { pointee_ty } => {
                    if let Some(arg) = arguments.first() {
                        // Argument type should match pointee_ty
                        ctx.register_type(pointee_ty);
                        ctx.equate(arg.value.ty.id(), pointee_ty.id(), arg.span.clone());
                    }
                },

                LangIntrinsic::PtrFromAddress { pointee_ty } => {
                    // Argument is an integer, pointee_ty is the type parameter
                    // Just register it for resolution
                    ctx.register_type(pointee_ty);
                },

                LangIntrinsic::PtrNull { pointee_ty } => {
                    // No arguments, but register pointee_ty for resolution
                    ctx.register_type(pointee_ty);
                },

                LangIntrinsic::SizeOf { ty } | LangIntrinsic::AlignOf { ty } => {
                    // No arguments, but register ty for resolution
                    ctx.register_type(ty);
                },

                // Numeric binary intrinsics - constrain both arguments to match the primitive type
                LangIntrinsic::IntBinary { primitive, .. }
                | LangIntrinsic::IntBinarySigned { primitive, .. }
                | LangIntrinsic::IntBinaryUnsigned { primitive, .. }
                | LangIntrinsic::FloatBinary { primitive, .. } => {
                    let prim_ty = primitive.to_ty(expr.span.clone());
                    ctx.register_type(&prim_ty);
                    for arg in arguments {
                        ctx.equate(arg.value.ty.id(), prim_ty.id(), arg.span.clone());
                    }
                },

                // Numeric unary intrinsics - constrain argument to match the primitive type
                LangIntrinsic::IntUnary { primitive, .. }
                | LangIntrinsic::FloatUnary { primitive, .. } => {
                    let prim_ty = primitive.to_ty(expr.span.clone());
                    ctx.register_type(&prim_ty);
                    if let Some(arg) = arguments.first() {
                        ctx.equate(arg.value.ty.id(), prim_ty.id(), arg.span.clone());
                    }
                },

                // Float predicates (isNan, isInfinite) - constrain argument to float type
                LangIntrinsic::FloatPred { primitive, .. } => {
                    let prim_ty = primitive.to_ty(expr.span.clone());
                    ctx.register_type(&prim_ty);
                    if let Some(arg) = arguments.first() {
                        ctx.equate(arg.value.ty.id(), prim_ty.id(), arg.span.clone());
                    }
                },

                LangIntrinsic::PtrOffset => {
                    // Argument 1: Pointer[T]
                    // Argument 2: lang.i64
                    // Returns: Pointer[T]
                    if let Some(ptr_arg) = arguments.first()
                        && let TyKind::Pointer(pointee) = ptr_arg.value.ty.kind()
                    {
                        ctx.register_type(pointee);
                    }
                    if let Some(offset_arg) = arguments.get(1) {
                        use kestrel_semantic_tree::ty::IntBits;
                        let i64_ty = Ty::int(IntBits::I64, offset_arg.span.clone());
                        ctx.register_type(&i64_ty);
                        ctx.equate(
                            offset_arg.value.ty.id(),
                            i64_ty.id(),
                            offset_arg.span.clone(),
                        );
                    }
                },

                LangIntrinsic::PtrToAddress => {
                    // Argument 1: Pointer[T]
                    // Returns: lang.i64
                    if let Some(arg) = arguments.first()
                        && let TyKind::Pointer(pointee) = arg.value.ty.kind()
                    {
                        ctx.register_type(pointee);
                    }
                },

                LangIntrinsic::PtrIsNull => {
                    // Argument 1: Pointer[T]
                    // Returns: lang.i1
                    if let Some(arg) = arguments.first()
                        && let TyKind::Pointer(pointee) = arg.value.ty.kind()
                    {
                        ctx.register_type(pointee);
                    }
                },

                // Other intrinsics don't have type parameters that need constraint generation
                _ => {},
            }
        },

        // Language intrinsic reference - no constraints needed
        ExprKind::LangIntrinsicRef(_) => {},

        // Subscript call - process receiver and arguments
        ExprKind::SubscriptCall {
            receiver,
            arguments,
            ..
        } => {
            generate_expression_constraints(ctx, receiver);
            for arg in arguments {
                generate_expression_constraints(ctx, &arg.value);
            }
        },

        ExprKind::ProtocolPropertyAccess { receiver, .. } => {
            // Constraints already handled during resolution
            // Just recurse into receiver
            generate_expression_constraints(ctx, receiver);
        },

        ExprKind::Error => {},
    }
}

#[cfg(test)]
mod tests {
    // Integration tests would require setting up a full semantic model
}
