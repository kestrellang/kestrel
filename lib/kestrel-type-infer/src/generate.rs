//! Constraint generation: walk HirBody once, emitting constraints for every node.
//!
//! This is a one-pass walk that creates TyVars for every expression,
//! statement binding, and local variable, then emits constraints
//! capturing the type relationships between them.

use kestrel_ast_builder::{Callable, InitEffect, Name, NodeKind, TypeParams};
use kestrel_hecs::Entity;
use kestrel_hir::Builtin;
use kestrel_hir::body::*;
use kestrel_hir::ty::HirTy;
use kestrel_hir_lower::{LowerCallableReturnType, LowerCallableTypes, LowerTypeAnnotation};
use kestrel_span::Span;

use crate::constraint::{CallArg, Constraint, labels_match};
use crate::ctx::InferCtx;
use crate::error::InferError;
use crate::ty::{LiteralKind, TyKind, TySlot, TyVar};
use crate::unify;

// ===== Entry point =====

/// Generate constraints for an entire HirBody.
/// Creates TyVars for params and return type, walks all statements and tail expr.
pub fn generate(ctx: &mut InferCtx<'_>, hir: &HirBody, param_types: &[TyVar], return_ty: TyVar) {
    // Bind param locals to their TyVars
    for (&param_local, &param_tv) in hir.params.iter().zip(param_types.iter()) {
        ctx.local_types.insert(param_local, param_tv);
    }

    // Set return type
    ctx.return_ty = return_ty;

    // Walk statements
    for &stmt_id in &hir.statements {
        gen_stmt(ctx, hir, stmt_id);
    }

    // Tail expression flows to return type. Skip the coerce when the tail
    // is a control-flow construct that may fall through to unit — the
    // exhaustive_return analyzer (E001) reports that more precisely, and
    // the redundant E100 here would mask it at diagnostic-match time.
    if let Some(tail) = hir.tail_expr {
        let tail_tv = gen_expr(ctx, hir, tail);
        if tail_is_exhaustive(hir, tail) {
            let span = expr_span(hir, tail);
            ctx.coerce(tail_tv, ctx.return_ty, tail, span);
        }
    }

    // Report type parameters used as standalone values (not consumed by MethodCall/Call)
    let stale_defs: Vec<Span> = ctx.type_param_defs.drain().map(|(_, s)| s).collect();
    for span in stale_defs {
        ctx.report_error(InferError::TypeParamAsValue { span });
    }
}

// ===== Expression generation =====

/// Generate constraints for an expression, returning its TyVar.
fn gen_expr(ctx: &mut InferCtx<'_>, hir: &HirBody, id: HirExprId) -> TyVar {
    let tv = match &hir.exprs[id] {
        // === Literals ===
        HirExpr::Literal { value, .. } => match value {
            HirLiteral::Integer(_) => ctx.fresh_literal(LiteralKind::Integer),
            HirLiteral::Float(_) => ctx.fresh_literal(LiteralKind::Float),
            HirLiteral::String { .. } => ctx.fresh_literal(LiteralKind::String),
            HirLiteral::Char(_) => ctx.fresh_literal(LiteralKind::Char),
            HirLiteral::Bool(_) => ctx.fresh_literal(LiteralKind::Bool),
            HirLiteral::Null => ctx.fresh_literal(LiteralKind::Null),
        },

        // === References ===
        HirExpr::Local(local_id, _) => {
            // Return the TyVar assigned when this local was declared
            ctx.local_types.get(local_id).copied().unwrap_or_else(|| {
                let local = &hir.locals[*local_id];
                kestrel_debug::ktrace!(
                    "hir-lower",
                    "missing local type for {:?} (local_id {:?})",
                    local.name,
                    local_id
                );
                ctx.report_error(InferError::FromHir {
                    span: expr_span(hir, id),
                })
            })
        },

        HirExpr::Def(entity, explicit_type_args, span) => {
            // Read the entity's type from the world via the resolver.
            // For generic entities, this will instantiate fresh TyVars.
            // If explicit type args are provided (e.g., Pointer[UInt8]),
            // use them instead of fresh inference variables.
            let (tv, type_arg_vars) =
                instantiate_entity_with_args(ctx, *entity, explicit_type_args, span);
            // Record type arg vars so MIR lowering can retrieve the resolved types
            if !type_arg_vars.is_empty() {
                ctx.record_type_args(id, type_arg_vars, span.clone());
            }
            // Track type parameter references — invalid unless consumed by MethodCall/Call
            if ctx.query_ctx.get::<NodeKind>(*entity) == Some(&NodeKind::TypeParameter) {
                ctx.type_param_defs.insert(id, span.clone());
            }
            tv
        },

        // Overloaded function reference — can only appear as callee of Call
        HirExpr::OverloadSet { span, .. } => {
            let recv = ctx.fresh();
            ctx.report_error(InferError::AmbiguousMember {
                receiver: recv,
                name: "<overloaded function>".into(),
                span: span.clone(),
            })
        },

        // === Calls ===
        HirExpr::Call { callee, args, span } => {
            // Overloaded free function call: emit OverloadedCall constraint
            if let HirExpr::OverloadSet {
                candidates,
                type_args,
                ..
            } = &hir.exprs[*callee]
            {
                let candidates = candidates.clone();
                let type_args = type_args.clone();
                let arg_tvs = gen_call_args(ctx, hir, args);
                let result_tv = ctx.fresh();
                ctx.overloaded_call(candidates, type_args, arg_tvs, result_tv, id, span.clone());
                ctx.expr_types.insert(id, result_tv);
                return result_tv;
            }

            // Struct construction: Def(struct) used as callee. Free-standing
            // type aliases are already dereferenced by name resolution, so
            // `type C = Counter; C(42)` reaches here with Def(Counter).
            if let HirExpr::Def(entity, explicit_type_args, _) = &hir.exprs[*callee]
                && ctx.query_ctx.get::<NodeKind>(*entity) == Some(&NodeKind::Struct) {
                    let arg_tvs = gen_call_args(ctx, hir, args);
                    return gen_struct_init(ctx, *entity, explicit_type_args, &arg_tvs, id, span);
                }

            // Enum case construction: route through OverloadedCall for label checking
            if let HirExpr::Def(entity, _, _) = &hir.exprs[*callee]
                && ctx.query_ctx.get::<NodeKind>(*entity) == Some(&NodeKind::EnumCase)
                    && ctx.query_ctx.get::<Callable>(*entity).is_some()
                {
                    let arg_tvs = gen_call_args(ctx, hir, args);
                    let result_tv = ctx.fresh();
                    ctx.overloaded_call(
                        vec![*entity],
                        vec![],
                        arg_tvs,
                        result_tv,
                        id,
                        span.clone(),
                    );
                    return result_tv;
                }

            // Free-function call with a single Def callee: pre-check labels +
            // arity so mismatches surface as NoMatchingOverload (richer phrasing
            // the tests expect) instead of the generic "wrong number of arguments"
            // / "wrong label" from solve_call. Matching calls fall through to the
            // regular ctx.call path, which handles parent-entity type-param
            // substitution for path-qualified callees (e.g., Pointer[UInt8].nullPointer()).
            if let HirExpr::Def(entity, _, _) = &hir.exprs[*callee]
                && ctx.query_ctx.get::<NodeKind>(*entity) == Some(&NodeKind::Function)
                    && let Some(callable) = ctx.query_ctx.get::<Callable>(*entity) {
                        let arg_labels: Vec<Option<&str>> =
                            args.iter().map(|a| a.label.as_deref()).collect();
                        if !labels_match(&callable.params, &arg_labels) {
                            let name = ctx
                                .query_ctx
                                .get::<Name>(*entity)
                                .map(|n| n.0.clone())
                                .unwrap_or_else(|| "<fn>".into());
                            let span = span.clone();
                            ctx.type_param_defs.remove(callee);
                            let _ = gen_call_args(ctx, hir, args);
                            return ctx.report_error(InferError::NoMatchingOverload { name, span });
                        }
                    }

            // Emit Call constraint — solver dispatches based on callee type:
            // Function → unify params/return, Named → subscript resolution
            let callee_tv = gen_expr(ctx, hir, *callee);
            // Consuming a Def(TypeParameter) as a Call callee is valid (T() = init)
            ctx.type_param_defs.remove(callee);
            let arg_tvs = gen_call_args(ctx, hir, args);

            // If the callee is a known function returning Never (e.g., lang.panic),
            // use Never directly so divergence propagates through control flow.
            let result_tv = if let HirExpr::Def(entity, _, _) = &hir.exprs[*callee] {
                if let Some(HirTy::Never(_)) =
                    ctx.query_ctx.query(kestrel_hir_lower::LowerTypeAnnotation {
                        entity: *entity,
                        root: ctx.root,
                    })
                {
                    ctx.never()
                } else {
                    ctx.fresh()
                }
            } else {
                ctx.fresh()
            };

            ctx.call(callee_tv, arg_tvs, result_tv, id, span.clone());
            result_tv
        },

        HirExpr::MethodCall {
            receiver,
            method,
            type_args: explicit_targs,
            args,
            span,
        } => {
            let recv_tv = gen_expr(ctx, hir, *receiver);
            let arg_tvs = gen_call_args(ctx, hir, args);
            let result_tv = ctx.fresh();

            // A MethodCall whose receiver is a type-level reference (struct,
            // enum, protocol, type alias, type parameter) is a static call —
            // e.g. `Counter.getValue()`, `T.staticFn()`, `Pointer[UInt8].nullPointer()`.
            // solve_member uses this to reject instance methods called as static.
            let is_static_ctx = matches!(
                &hir.exprs[*receiver],
                HirExpr::Def(entity, _, _) if matches!(
                    ctx.query_ctx.get::<NodeKind>(*entity),
                    Some(&NodeKind::Struct)
                        | Some(&NodeKind::Enum)
                        | Some(&NodeKind::Protocol)
                        | Some(&NodeKind::TypeAlias)
                        | Some(&NodeKind::TypeParameter)
                )
            );

            let has_explicit = explicit_targs.as_ref().is_some_and(|a| !a.is_empty());

            // Consuming a Def(TypeParameter) as a MethodCall receiver is valid
            if is_static_ctx {
                ctx.type_param_defs.remove(receiver);
            }

            let Some(method_str) = method.as_str() else {
                // Parser already reported "expected identifier after `.`";
                // short-circuit to Error so downstream constraints absorb.
                ctx.poison(result_tv);
                return result_tv;
            };

            if is_static_ctx {
                let ext_args = if has_explicit {
                    explicit_targs.clone().unwrap()
                } else {
                    Vec::new()
                };
                ctx.member_static(
                    recv_tv,
                    method_str,
                    arg_tvs,
                    result_tv,
                    id,
                    true,
                    ext_args,
                    span.clone(),
                );
            } else if has_explicit {
                ctx.member_with_type_args(
                    recv_tv,
                    method_str,
                    arg_tvs,
                    result_tv,
                    id,
                    true,
                    explicit_targs.clone().unwrap(),
                    span.clone(),
                );
            } else {
                ctx.member(
                    recv_tv,
                    method_str,
                    arg_tvs,
                    result_tv,
                    id,
                    true,
                    span.clone(),
                );
            }
            result_tv
        },

        HirExpr::ProtocolCall {
            receiver,
            protocol,
            method,
            args,
            span,
            ..
        } => {
            let recv_tv = gen_expr(ctx, hir, *receiver);
            let arg_tvs = gen_call_args(ctx, hir, args);
            let result_tv = ctx.fresh();

            // Receiver must conform to the protocol. When this ProtocolCall
            // sits inside a Sugar wrapper (the desugaring's primary call),
            // emit a poisoning Conforms so a non-conforming receiver poisons
            // recv_tv. The downstream Member constraint then sees an Error
            // receiver and absorbs without emitting "no member" cascades.
            if ctx.poison_protocol_call_recv_on_failure.contains(&id) {
                ctx.conforms_poisoning(recv_tv, *protocol, span.clone());
            } else {
                ctx.conforms(recv_tv, *protocol, span.clone());
            }
            let Some(method_str) = method.as_str() else {
                ctx.poison(result_tv);
                return result_tv;
            };
            // Resolve method on the protocol
            ctx.member(
                recv_tv,
                method_str,
                arg_tvs,
                result_tv,
                id,
                true,
                span.clone(),
            );
            result_tv
        },

        // === Member access ===
        HirExpr::Field {
            base, name, span, ..
        } => {
            let base_tv = gen_expr(ctx, hir, *base);
            let result_tv = ctx.fresh();

            // Field access through a type-param ref (T.staticProp) is a static
            // protocol-property access — analogous to T.method() in MethodCall.
            // Consume the Def(TypeParameter) so it isn't flagged as a stray
            // type-param-as-value at end of inference.
            if matches!(
                &hir.exprs[*base],
                HirExpr::Def(entity, _, _) if ctx.query_ctx.get::<NodeKind>(*entity)
                    == Some(&NodeKind::TypeParameter)
            ) {
                ctx.type_param_defs.remove(base);
            }

            match name {
                HirName::Name(name_str) => {
                    // Member constraint with no args (field/property access)
                    ctx.member(
                        base_tv,
                        name_str,
                        vec![],
                        result_tv,
                        id,
                        false,
                        span.clone(),
                    );
                },
                HirName::Missing => {
                    // Parser already reported "expected identifier after `.`".
                    // Silently bind the result to Error so downstream constraints
                    // absorb without cascading "name not found" diagnostics.
                    ctx.poison(result_tv);
                },
            }
            result_tv
        },

        HirExpr::TupleIndex {
            base, index, span, ..
        } => {
            let base_tv = gen_expr(ctx, hir, *base);
            let result_tv = ctx.fresh();
            ctx.constraints
                .push(crate::constraint::Constraint::TupleIndex {
                    tuple: base_tv,
                    index: *index as usize,
                    result: result_tv,
                    span: span.clone(),
                });
            result_tv
        },

        // === Implicit member (.CaseName) ===
        HirExpr::ImplicitMember { name, args, span } => {
            let arg_tvs = args
                .as_ref()
                .map(|a| gen_call_args(ctx, hir, a))
                .unwrap_or_default();
            let result_tv = ctx.fresh();

            let Some(name_str) = name.as_str() else {
                ctx.poison(result_tv);
                return result_tv;
            };
            // Implicit constraint: resolved against expected type
            ctx.implicit(result_tv, name_str, arg_tvs, result_tv, id, span.clone());
            result_tv
        },

        // === Control flow ===
        HirExpr::If {
            condition,
            then_body,
            else_body,
            span,
        } => {
            // Condition type is not constrained during inference.
            // Like lib1, validation of BooleanConditional conformance
            // happens in a later pass. This avoids i1/Bool mismatches
            // when stdlib code uses lang intrinsics directly in conditions.
            let _cond_tv = gen_expr(ctx, hir, *condition);

            let then_tv = gen_block(ctx, hir, then_body);

            if let Some(else_block) = else_body {
                let else_tv = gen_block(ctx, hir, else_block);
                let result_tv = ctx.fresh();
                // Both branches must agree — use block value spans so
                // errors point at the mismatched expression, not the if keyword
                let then_span = block_value_span(hir, then_body).unwrap_or_else(|| span.clone());
                ctx.equal(then_tv, result_tv, then_span);
                // Guard desugars to `if cond {} else { body }` where the
                // else block is required to diverge. Don't equate its value
                // type with the empty then branch — the guard divergence
                // analyzer is responsible for enforcing divergence.
                if !is_guard_if(hir, id) {
                    let else_span =
                        block_value_span(hir, else_block).unwrap_or_else(|| span.clone());
                    ctx.equal(else_tv, result_tv, else_span);
                }
                result_tv
            } else {
                // No else: expression has unit type
                ctx.tuple(vec![])
            }
        },

        HirExpr::Match {
            scrutinee,
            arms,
            source,
            ..
        } => {
            let scrut_tv = gen_expr(ctx, hir, *scrutinee);
            let result_tv = ctx.fresh();

            // Empty match has no arms to pin the result type. An analyzer
            // reports the `empty match` diagnostic — poison the result so a
            // cascading "could not infer type" doesn't obscure the real error.
            if arms.is_empty() {
                ctx.poison(result_tv);
            }

            for arm in arms {
                // Pattern constrains scrutinee type
                gen_pat(ctx, hir, arm.pattern, scrut_tv, *source);

                // Guard must be bool
                if let Some(guard) = arm.guard {
                    // Guard type validated later (same as if-condition)
                    let _guard_tv = gen_expr(ctx, hir, guard);
                }

                // Body must match result type — use the arm body's span so
                // mismatch diagnostics point at the offending arm, not the
                // match keyword. Never-arms don't pin the result (see the
                // `Never` branch in `unify`); if every arm turns out to be
                // Never, `default_never_fallback` settles it.
                let body_tv = gen_expr(ctx, hir, arm.body);
                let body_span = expr_span(hir, arm.body);
                ctx.equal(body_tv, result_tv, body_span);
            }
            result_tv
        },

        HirExpr::Loop { label, body, .. } => {
            let break_tv = ctx.fresh();
            ctx.loop_break_tys.push((label.clone(), break_tv));
            gen_block(ctx, hir, body);
            ctx.loop_break_tys.pop();
            // If no break reached this loop, break_tv stays Unresolved
            // and never-fallback defaults it to Never.
            ctx.never_fallback_targets.insert(break_tv);
            break_tv
        },

        HirExpr::Break { label, .. } => {
            let unit_tv = ctx.tuple(vec![]);
            if let Some((_lbl, break_tv)) = label
                .as_ref()
                .and_then(|l| ctx.loop_break_tys.iter().rev().find(|(k, _)| k.as_deref() == Some(l)))
                .or_else(|| ctx.loop_break_tys.last())
                .cloned()
            {
                let _ = unify::unify(ctx, unit_tv, break_tv);
            }
            ctx.never()
        },

        HirExpr::Continue { .. } => ctx.never(),

        HirExpr::Return { value, span } => {
            if let Some(val) = value {
                let val_tv = gen_expr(ctx, hir, *val);
                ctx.coerce(val_tv, ctx.return_ty, id, span.clone());
            } else {
                // Bare return — coerce unit against return type so
                // `return` in a non-void function is a type mismatch
                let unit_tv = ctx.tuple(vec![]);
                ctx.coerce(unit_tv, ctx.return_ty, id, span.clone());
            }
            ctx.never()
        },

        // === Assignment ===
        HirExpr::Assign {
            target,
            value,
            span,
        } => {
            let target_tv = gen_expr(ctx, hir, *target);
            let value_tv = gen_expr(ctx, hir, *value);
            ctx.coerce(value_tv, target_tv, id, span.clone());
            ctx.tuple(vec![]) // assignment returns unit
        },

        // === Closures ===
        HirExpr::Closure { params, body, .. } => gen_closure(ctx, hir, params, body),

        // === Aggregates ===
        HirExpr::Array { elements, span } => {
            // Tie elements to `target.Element` via an Associated constraint, so
            // when the array literal is coerced into a custom target type (e.g.
            // `MyList` with `type Element = Int64`) the element type flows back
            // to each literal element. Falls back to fresh on the default path.
            let arr_tv = ctx.fresh_literal(LiteralKind::Array);
            let elem_tv = ctx.fresh();
            ctx.associated(arr_tv, "Element", elem_tv, span.clone());
            // Bidirectional hint: if the enclosing `let` annotated this array
            // with `Array[E]`, pre-seed `elem_tv` with `E` before element
            // constraints run. This way each element is compared against the
            // target element type, not against whatever literal kind the first
            // element happens to have.
            if let Some(hint_elem) = ctx.expected_array_elem.take() {
                ctx.equal(elem_tv, hint_elem, span.clone());
            }
            for &e in elements {
                let e_tv = gen_expr(ctx, hir, e);
                let e_span = expr_span(hir, e);
                // Order (elem_tv, e_tv) so diagnostics read "expected <target>
                // got <element>" rather than the reverse.
                ctx.equal(elem_tv, e_tv, e_span);
            }
            arr_tv
        },

        HirExpr::Dict { entries, span } => {
            let dict_tv = ctx.fresh_literal(LiteralKind::Dictionary);
            let key_tv = ctx.fresh();
            let val_tv = ctx.fresh();
            ctx.associated(dict_tv, "Key", key_tv, span.clone());
            ctx.associated(dict_tv, "Value", val_tv, span.clone());
            let expected_entry = ctx.expected_dict_entry.take();
            if let Some((hint_key, hint_val)) = expected_entry {
                ctx.equal(key_tv, hint_key, span.clone());
                ctx.equal(val_tv, hint_val, span.clone());
            }
            for entry in entries {
                let k = gen_expr(ctx, hir, entry.key);
                let v = gen_expr(ctx, hir, entry.value);
                let key_span = expr_span(hir, entry.key);
                let value_span = expr_span(hir, entry.value);
                let expected_key = expected_entry
                    .map(|(hint_key, _)| hint_key)
                    .unwrap_or(key_tv);
                let expected_value = expected_entry
                    .map(|(_, hint_value)| hint_value)
                    .unwrap_or(val_tv);
                if !emit_dict_literal_acceptance_error(
                    ctx,
                    hir,
                    expected_key,
                    entry.key,
                    key_span.clone(),
                ) {
                    ctx.equal(key_tv, k, key_span);
                }
                if !emit_dict_literal_acceptance_error(
                    ctx,
                    hir,
                    expected_value,
                    entry.value,
                    value_span.clone(),
                ) {
                    ctx.equal(val_tv, v, value_span);
                }
            }
            dict_tv
        },

        HirExpr::Tuple { elements, span: _ } => {
            let elem_tvs: Vec<TyVar> = elements.iter().map(|&e| gen_expr(ctx, hir, e)).collect();
            ctx.tuple(elem_tvs)
        },

        // Block expression: execute stmts, result is the tail expr
        HirExpr::Block { body, .. } => gen_block(ctx, hir, body),

        HirExpr::Error { span } => ctx.report_error(InferError::FromHir { span: span.clone() }),

        // Sugar wrapper: kind-aware primary-constraint emission for cascade
        // suppression. The relevant inner ProtocolCall expr_id is recorded in
        // `poison_protocol_call_recv_on_failure` BEFORE recursing, so when
        // `gen_expr` reaches that ProtocolCall it emits a poisoning Conforms.
        // Type-of(Sugar) == type-of(inner) (structurally transparent).
        HirExpr::Sugar { kind, inner, .. } => {
            mark_sugar_primary(ctx, hir, *kind, *inner);
            gen_expr(ctx, hir, *inner)
        },
    };

    // Record the type for this expression
    ctx.expr_types.insert(id, tv);
    tv
}

/// Record which inner `ProtocolCall` expr_id is the "primary" for a Sugar
/// wrapper, so the ProtocolCall arm of `gen_expr` emits a poisoning
/// `Conforms` constraint (cascade suppression). Walks the inner shape
/// per-kind; bails silently if the shape doesn't match (defensive — desugar
/// should always produce the canonical shape, but a malformed inner from
/// error recovery shouldn't crash inference).
fn mark_sugar_primary(
    ctx: &mut InferCtx<'_>,
    hir: &HirBody,
    kind: kestrel_hir::body::SugarKind,
    inner: HirExprId,
) {
    use kestrel_hir::body::SugarKind;
    match kind {
        SugarKind::ForLoop => {
            // inner is `Block { stmts: [Let { value: Some(iter_pcall) }], tail_expr: Some(loop) }`.
            // The iter_pcall is the IterableProtocol "iter" call we want to poison.
            let HirExpr::Block { body, .. } = &hir.exprs[inner] else {
                return;
            };
            let Some(first_stmt) = body.stmts.first() else {
                return;
            };
            let HirStmt::Let {
                value: Some(pcall_id),
                ..
            } = &hir.stmts[*first_stmt]
            else {
                return;
            };
            ctx.poison_protocol_call_recv_on_failure.insert(*pcall_id);
        },
        SugarKind::Try => {
            // inner is `Match { scrutinee: tryExtract_pcall, ..., source: TryOp }`.
            let HirExpr::Match { scrutinee, .. } = &hir.exprs[inner] else {
                return;
            };
            ctx.poison_protocol_call_recv_on_failure.insert(*scrutinee);
        },
        SugarKind::CompoundAssign => {
            // inner is the addAssign ProtocolCall (or HirExpr::Error if the
            // AST place check rejected the LHS at desugar time).
            ctx.poison_protocol_call_recv_on_failure.insert(inner);
        },
        SugarKind::StringInterpolation => {
            // inner is Block { stmts: [Let($dsi, init_call), ...], tail_expr: build_call }.
            // If the DSI init fails (stdlib missing), poison so append/build errors absorb.
            let HirExpr::Block { body, .. } = &hir.exprs[inner] else {
                return;
            };
            if let Some(first_stmt) = body.stmts.first()
                && let HirStmt::Let {
                    value: Some(init_call),
                    ..
                } = &hir.stmts[*first_stmt]
            {
                ctx.poison_protocol_call_recv_on_failure.insert(*init_call);
            }
        },
    }
}

// ===== Statement generation =====

fn gen_stmt(ctx: &mut InferCtx<'_>, hir: &HirBody, id: HirStmtId) {
    match &hir.stmts[id] {
        HirStmt::Let {
            local,
            ty,
            value,
            span,
        } => {
            let local_tv = if let Some(ty) = ty {
                // Annotated: convert HirTy -> TyVar
                lower_hir_ty(ctx, ty)
            } else {
                // Unannotated: fresh TyVar, inferred from value
                ctx.fresh()
            };

            ctx.local_types.insert(*local, local_tv);

            if let Some(val) = value {
                // Bidirectional hint: if the annotation is `Array[E]` and the
                // RHS is an array literal, feed `E` down so the array's element
                // TyVar is pinned to the target before element equates run.
                let prev_hint = ctx.expected_array_elem;
                if ty.is_some() && matches!(&hir.exprs[*val], HirExpr::Array { .. }) {
                    ctx.expected_array_elem = extract_array_elem_hint(ctx, local_tv);
                }
                let prev_dict_hint = ctx.expected_dict_entry;
                if ty.is_some() && matches!(&hir.exprs[*val], HirExpr::Dict { .. }) {
                    ctx.expected_dict_entry = extract_dict_entry_hint(ctx, local_tv);
                }
                let val_tv = gen_expr(ctx, hir, *val);
                ctx.expected_array_elem = prev_hint;
                ctx.expected_dict_entry = prev_dict_hint;
                // Value flows to the binding (allows promotion)
                ctx.coerce(val_tv, local_tv, *val, span.clone());
            }
        },

        HirStmt::Expr { expr, .. } => {
            gen_expr(ctx, hir, *expr);
        },

        HirStmt::Deinit { .. } => {
            // Deinit has no type semantics for inference
        },
    }
}

// ===== Pattern generation =====

/// Generate constraints for a pattern given the type of the scrutinee.
///
/// `source` identifies the enclosing `HirExpr::Match`. For
/// `MatchSource::ParamDestructure`, a tuple-pattern arity mismatch is left to
/// the `param_pattern` analyzer (E111) — skipping the scrutinee/pattern
/// equate here prevents the cascading generic "type mismatch" diagnostic.
fn gen_pat(
    ctx: &mut InferCtx<'_>,
    hir: &HirBody,
    pat_id: HirPatId,
    scrutinee_tv: TyVar,
    source: MatchSource,
) {
    match &hir.pats[pat_id] {
        HirPat::Wildcard { .. } => {
            // No constraint — matches anything
        },

        HirPat::Binding { local, .. } => {
            // Bind local to the scrutinee type
            ctx.local_types.insert(*local, scrutinee_tv);
        },

        HirPat::Literal { value, span, .. } => {
            let lit_tv = literal_to_tyvar(ctx, value);
            ctx.equal(lit_tv, scrutinee_tv, span.clone());
        },

        HirPat::Tuple {
            prefix,
            has_rest,
            suffix,
            span,
        } => {
            let prefix_tvs: Vec<TyVar> = prefix.iter().map(|_| ctx.fresh()).collect();
            let suffix_tvs: Vec<TyVar> = suffix.iter().map(|_| ctx.fresh()).collect();

            if *has_rest {
                // Defer until scrutinee resolves — we need the actual tuple arity
                ctx.constraints.push(Constraint::TupleRestPat {
                    scrutinee: scrutinee_tv,
                    prefix_tys: prefix_tvs.clone(),
                    suffix_tys: suffix_tvs.clone(),
                    span: span.clone(),
                });
            } else {
                // Fixed arity: equate scrutinee against tuple of exactly these
                // elements, unless the pattern analyzer will flag the arity
                // (E111 for params, E314 for match arms) — in that case skip
                // the equate to avoid a duplicate generic type-mismatch
                // diagnostic on the same span.
                let pat_arity = prefix.len();
                let suppress = matches!(
                    ctx.slot(scrutinee_tv),
                    TySlot::Resolved(TyKind::Tuple(elems)) if elems.len() != pat_arity
                );
                if !suppress {
                    let tuple_tv = ctx.tuple(prefix_tvs.clone());
                    ctx.equal(scrutinee_tv, tuple_tv, span.clone());
                }
            }

            for (&elem_pat, elem_tv) in prefix.iter().zip(prefix_tvs) {
                gen_pat(ctx, hir, elem_pat, elem_tv, source);
            }
            for (&elem_pat, elem_tv) in suffix.iter().zip(suffix_tvs) {
                gen_pat(ctx, hir, elem_pat, elem_tv, source);
            }
        },

        HirPat::Variant { entity, args, span } => {
            gen_variant_pat(ctx, hir, *entity, args, scrutinee_tv, span, source);
        },

        HirPat::ImplicitVariant { name, args, span } => {
            // Skip pattern-level resolution when the case name is missing —
            // the parser already reported the gap. Walking sub-patterns
            // would still emit unification noise, so leave them unresolved.
            if let Some(name_str) = name.as_str() {
                gen_implicit_variant_pat(ctx, hir, name_str, args, scrutinee_tv, span, source);
            }
        },

        HirPat::Struct {
            entity,
            fields,
            span,
            ..
        } => {
            gen_struct_pat(ctx, hir, *entity, fields, scrutinee_tv, span, source);
        },

        HirPat::Or { alternatives, .. } => {
            for &alt in alternatives {
                gen_pat(ctx, hir, alt, scrutinee_tv, source);
            }
        },

        HirPat::Range { span, .. } => {
            // Range patterns constrain the scrutinee to be the literal type
            // For now, just accept — range pattern types are validated later
            let _ = span;
        },

        HirPat::At {
            binding,
            subpattern,
            ..
        } => {
            // Bind the whole matched value to the local, then constrain via subpattern
            ctx.local_types.insert(*binding, scrutinee_tv);
            gen_pat(ctx, hir, *subpattern, scrutinee_tv, source);
        },

        HirPat::Array {
            prefix,
            rest,
            suffix,
            span,
        } => {
            // Array patterns accept both `Array[T]` and `Slice[T]` scrutinees.
            // If the scrutinee is already resolved to `Slice[T]`, take the
            // element type from there; otherwise default to equating with
            // `Array[elem_tv]` (preserves existing behavior for generic /
            // unresolved scrutinees).
            let slice_entity = ctx.resolver.builtin(kestrel_hir::Builtin::SliceStruct);
            let elem_tv = {
                let already_slice = if let Some(slice_ent) = slice_entity {
                    matches!(
                        ctx.slot(scrutinee_tv),
                        TySlot::Resolved(k) if k.entity() == Some(slice_ent)
                    )
                } else {
                    false
                };

                if already_slice {
                    // Reuse the scrutinee's element type arg directly.
                    let first_arg = match ctx.slot(scrutinee_tv) {
                        TySlot::Resolved(k) => k.args().first().copied(),
                        _ => unreachable!(),
                    };
                    first_arg.unwrap_or_else(|| ctx.fresh())
                } else {
                    let elem_tv = ctx.fresh();
                    if let Some(array_entity) = ctx
                        .resolver
                        .builtin(kestrel_hir::Builtin::DefaultArrayLiteralType)
                    {
                        let array_tv = ctx.named(array_entity, vec![elem_tv]);
                        ctx.equal(scrutinee_tv, array_tv, span.clone());
                    }
                    elem_tv
                }
            };

            // Equate each prefix/suffix element pattern against elem_tv
            for &elem_pat in prefix.iter().chain(suffix.iter()) {
                let pat_tv = ctx.fresh();
                ctx.equal(pat_tv, elem_tv, span.clone());
                gen_pat(ctx, hir, elem_pat, pat_tv, source);
            }

            // Named rest binding → `Slice[elem_tv]` local.
            if let Some(Some(local)) = rest {
                if let Some(slice_ent) = slice_entity {
                    let slice_tv = ctx.named(slice_ent, vec![elem_tv]);
                    ctx.local_types.insert(*local, slice_tv);
                } else {
                    // No SliceStruct builtin available — fall back to a fresh
                    // TyVar so at least the local is resolvable.
                    let tv = ctx.fresh();
                    ctx.local_types.insert(*local, tv);
                }
            }
        },

        HirPat::Error { .. } => { /* swallow */ },
    }
}

// ===== Struct construction =====

/// Generate constraints for a struct constructor call.
/// Finds a matching init (by arity + label pattern) or uses memberwise from fields.
fn gen_struct_init(
    ctx: &mut InferCtx<'_>,
    struct_entity: Entity,
    explicit_type_args: &[HirTy],
    args: &[CallArg],
    expr_id: HirExprId,
    span: &Span,
) -> TyVar {
    let qctx = ctx.query_ctx;
    let root = ctx.root;

    // Build the result type: struct with fresh type args
    let type_params: Vec<Entity> = qctx
        .get::<TypeParams>(struct_entity)
        .map(|tp| tp.0.clone())
        .unwrap_or_default();
    let fresh_args: Vec<TyVar> = type_params.iter().map(|_| ctx.fresh()).collect();

    // Constrain fresh type args with explicit type args (e.g., Array[Int64]())
    if !explicit_type_args.is_empty() {
        for (i, (&fresh_tv, &param_entity)) in fresh_args.iter().zip(type_params.iter()).enumerate()
        {
            if let Some(hir_ty) = explicit_type_args.get(i) {
                let explicit_tv = lower_hir_ty(ctx, hir_ty);
                ctx.equal(fresh_tv, explicit_tv, span.clone());
            } else if let Some(default_ty) = qctx.query(LowerTypeAnnotation {
                entity: param_entity,
                root,
            }) {
                // Apply default type param (e.g., Set[Int64]() → H defaults to DefaultHasher)
                let default_tv = lower_hir_ty(ctx, &default_ty);
                ctx.equal(fresh_tv, default_tv, span.clone());
            }
        }
    }

    let result_tv = ctx.named(struct_entity, fresh_args.clone());

    // Substitution map: struct type params → fresh TyVars.
    // Used to instantiate init param/field types with the right type vars.
    let struct_subs: Vec<(Entity, TyVar)> = type_params
        .iter()
        .zip(fresh_args.iter())
        .map(|(&e, &tv)| (e, tv))
        .collect();

    // Collect arg labels for init matching
    let arg_labels: Vec<Option<&str>> = args.iter().map(|a| a.label.as_deref()).collect();

    // Find initializer children (direct + from extensions)
    let children = qctx.children_of(struct_entity).to_vec();
    let mut inits: Vec<Entity> = children
        .iter()
        .filter(|&&c| qctx.get::<NodeKind>(c) == Some(&NodeKind::Initializer))
        .copied()
        .collect();

    // Also search extensions for init functions
    let extensions = qctx.query(kestrel_name_res::ExtensionsFor {
        target: struct_entity,
        root,
    });
    for ext in &extensions {
        for &child in qctx.children_of(*ext) {
            if qctx.get::<NodeKind>(child) == Some(&NodeKind::Initializer) {
                inits.push(child);
            }
        }
    }

    // Track the matched init entity for effect wrapping
    let mut matched_init: Option<Entity> = None;

    if !inits.is_empty() {
        // Collect all inits matching by arity + label pattern
        let matched: Vec<Entity> = inits
            .iter()
            .copied()
            .filter(|&init| {
                let Some(callable) = qctx.get::<Callable>(init) else {
                    return false;
                };
                labels_match(&callable.params, &arg_labels)
            })
            .collect();

        // Only emit arg constraints when exactly one init matches.
        // Multiple matches (e.g. Int64.init(from: Int8/UInt8/...)) need
        // type-directed disambiguation which happens during solving.
        if let [init] = matched.as_slice() {
            let init = *init;
            matched_init = Some(init);
            // Record which init was selected so MIR lowering can use the init entity
            ctx.resolutions.insert(expr_id, init);
            // Build subs that includes both struct type params AND init's own type params
            let init_type_params: Vec<Entity> = qctx
                .get::<TypeParams>(init)
                .map(|tp| tp.0.clone())
                .unwrap_or_default();
            let init_fresh: Vec<TyVar> = init_type_params.iter().map(|_| ctx.fresh()).collect();
            // Record the COMPLETE type-arg list (struct params first, then the
            // init's own method-level params) so MIR lowering sees the full
            // shape expected by the init function. Init functions inherit
            // their parent struct's type_params as their leading MIR
            // type_params — the recorded list has to match that layout exactly,
            // otherwise MIR lowering will silently double-count via the
            // prepend-struct-args fallback in emit_call_maybe_init.
            if !fresh_args.is_empty() || !init_fresh.is_empty() {
                let mut all: Vec<TyVar> = fresh_args.clone();
                all.extend(init_fresh.iter().copied());
                ctx.record_type_args(expr_id, all, span.clone());
            }
            let mut init_subs = struct_subs.clone();
            for (&e, &tv) in init_type_params.iter().zip(init_fresh.iter()) {
                init_subs.push((e, tv));
            }

            emit_where_clause_constraints_with_subs(ctx, init, &init_subs, span);

            // Constrain args against the matched init's param types
            if let Some(param_tys) = qctx.query(LowerCallableTypes { entity: init, root }) {
                for (arg, param_ty) in args.iter().zip(param_tys.iter()) {
                    if let Some(hir_ty) = param_ty {
                        let param_tv = lower_hir_ty_with_subs(ctx, hir_ty, &init_subs);
                        ctx.coerce(arg.ty, param_tv, expr_id, span.clone());
                    }
                }
            }
        } else if matched.is_empty() {
            // No init matches the call's labels. Without this report, the
            // arg TyVars stay unresolved (no coerce constraint is emitted)
            // and downstream MIR lowering happily picks the first init
            // anyway, silently miscompiling primitive args as `ref`.
            let name = qctx
                .get::<Name>(struct_entity)
                .map(|n| n.0.clone())
                .unwrap_or_else(|| "<struct>".into());
            ctx.report_error(InferError::NoMatchingOverload {
                name,
                span: span.clone(),
            });
        } else {
            // Multiple label matches — emit Member constraint.
            // resolve_member will detect the ambiguity and try protocol-based
            // resolution, letting the solver disambiguate via type inference.
            let recv_tv = ctx.named(struct_entity, fresh_args.clone());
            let init_result = ctx.fresh(); // init return type (discarded — result is struct type)
            ctx.member(
                recv_tv,
                "init",
                args.to_vec(),
                init_result,
                expr_id,
                true,
                span.clone(),
            );
            ctx.expr_types.insert(expr_id, result_tv);
            return result_tv;
        }
    } else {
        // Memberwise init: match args against stored field types (in order).
        // `NodeKind::Field` also covers computed properties, so filter them
        // out via the `Computed` marker — memberwise init only takes storage.
        let fields: Vec<Entity> = children
            .iter()
            .filter(|&&c| {
                qctx.get::<NodeKind>(c) == Some(&NodeKind::Field)
                    && qctx.get::<kestrel_ast_builder::Computed>(c).is_none()
            })
            .copied()
            .collect();

        // Auto-generated memberwise inits require one labeled argument per
        // field, in declaration order, with the label matching the field name.
        // Mismatches here silently truncate via zip() below, so validate upfront.
        if args.len() != fields.len() {
            let struct_name = qctx
                .get::<Name>(struct_entity)
                .map(|n| n.0.clone())
                .unwrap_or_else(|| "<struct>".into());
            ctx.report_error(InferError::MemberwiseInitArity {
                struct_name,
                expected: fields.len(),
                got: args.len(),
                span: span.clone(),
            });
        } else {
            for (arg, &field) in args.iter().zip(fields.iter()) {
                let expected_label = qctx
                    .get::<Name>(field)
                    .map(|n| n.0.clone())
                    .unwrap_or_default();
                if arg.label.as_deref() != Some(expected_label.as_str()) {
                    let struct_name = qctx
                        .get::<Name>(struct_entity)
                        .map(|n| n.0.clone())
                        .unwrap_or_else(|| "<struct>".into());
                    ctx.report_error(InferError::MemberwiseInitLabel {
                        struct_name,
                        expected: expected_label,
                        got: arg.label.clone(),
                        span: span.clone(),
                    });
                }
            }
        }

        for (arg, &field) in args.iter().zip(fields.iter()) {
            if let Some(hir_ty) = qctx.query(LowerTypeAnnotation {
                entity: field,
                root,
            }) {
                let field_tv = lower_hir_ty_with_subs(ctx, &hir_ty, &struct_subs);
                ctx.coerce(arg.ty, field_tv, expr_id, span.clone());
            }
        }
    }

    // Wrap result through type operators for effectful inits (Self? or Self throws E)
    let final_tv = if let Some(init) = matched_init {
        wrap_init_result(ctx, init, result_tv, &struct_subs, span)
    } else {
        result_tv
    };

    ctx.expr_types.insert(expr_id, final_tv);
    final_tv
}

// ===== Init effect wrapping =====

/// Wrap a TyVar for an effectful init call site: `Self` → `Self?` or `Self throws E`.
///
/// Uses the init's `LowerTypeAnnotation` (e.g. `Optional[()]` or `Result[(), E]`) to get
/// the wrapper entity, then substitutes the inner type for the first type arg.
fn wrap_init_result(
    ctx: &mut InferCtx<'_>,
    init_entity: Entity,
    inner_tv: TyVar,
    struct_subs: &[(Entity, TyVar)],
    _span: &Span,
) -> TyVar {
    let qctx = ctx.query_ctx;
    let root = ctx.root;

    if qctx.get::<InitEffect>(init_entity).is_none() {
        return inner_tv;
    }

    // The init's TypeAnnotation is ()? or () throws E, lowered to HirTy with the
    // actual wrapper entity (Optional enum or Result enum). Replace the first type arg
    // (which is ()) with the struct type to get Self? or Self throws E.
    let Some(hir_ty) = qctx.query(LowerTypeAnnotation {
        entity: init_entity,
        root,
    }) else {
        return inner_tv;
    };

    match &hir_ty {
        HirTy::Enum { entity, args, .. } | HirTy::Struct { entity, args, .. } => {
            let mut wrapped_args = Vec::with_capacity(args.len());
            // First arg is () — replace with inner_tv (Self)
            wrapped_args.push(inner_tv);
            // Remaining args (e.g. error type E) — lower with struct subs
            for arg in args.iter().skip(1) {
                wrapped_args.push(lower_hir_ty_with_subs(ctx, arg, struct_subs));
            }
            ctx.named(*entity, wrapped_args)
        }
        _ => inner_tv,
    }
}

// ===== Helpers =====

/// Generate constraints for call arguments, returning CallArg list.
fn gen_call_args(ctx: &mut InferCtx<'_>, hir: &HirBody, args: &[HirCallArg]) -> Vec<CallArg> {
    args.iter()
        .map(|arg| {
            let ty = gen_expr(ctx, hir, arg.value);
            CallArg {
                label: arg.label.clone(),
                ty,
            }
        })
        .collect()
}

/// Generate constraints for a block (list of stmts + optional tail expr).
fn gen_block(ctx: &mut InferCtx<'_>, hir: &HirBody, block: &HirBlock) -> TyVar {
    for &stmt_id in &block.stmts {
        gen_stmt(ctx, hir, stmt_id);
    }
    if let Some(tail) = block.tail_expr {
        gen_expr(ctx, hir, tail)
    } else if block_diverges(hir, &block.stmts) {
        // Block ends in return/break/continue — type is Never (bottom)
        ctx.never()
    } else {
        ctx.tuple(vec![]) // void block = unit
    }
}

/// Whether the tail expression of a function body is guaranteed to either
/// produce a value of the return type or diverge. Returns `false` when it
/// may structurally fall through to unit (e.g. an `if` without `else`, or
/// a nested if-chain whose last arm is missing). Used to suppress the
/// redundant tail-return type-mismatch so E001 (`missing_return`) labels
/// the fault on its own.
fn tail_is_exhaustive(hir: &HirBody, id: HirExprId) -> bool {
    match &hir.exprs[id] {
        HirExpr::Return { .. } | HirExpr::Break { .. } | HirExpr::Continue { .. } => true,
        HirExpr::If {
            then_body,
            else_body,
            ..
        } => {
            let Some(else_body) = else_body else {
                return false;
            };
            block_is_exhaustive(hir, then_body) && block_is_exhaustive(hir, else_body)
        },
        HirExpr::Match { arms, .. } => {
            !arms.is_empty() && arms.iter().all(|a| tail_is_exhaustive(hir, a.body))
        },
        HirExpr::Block { body, .. } => block_is_exhaustive(hir, body),
        // Loops and other expressions either produce a value or are Never-typed;
        // either way the coerce is well-defined.
        _ => true,
    }
}

fn block_is_exhaustive(hir: &HirBody, block: &HirBlock) -> bool {
    if let Some(tail) = block.tail_expr {
        tail_is_exhaustive(hir, tail)
    } else {
        block_diverges(hir, &block.stmts)
    }
}

/// Check if a block's last statement is a diverging expression (return/break/continue).
/// Used so `{ return .None }` has type Never instead of unit.
fn block_diverges(hir: &HirBody, stmts: &[HirStmtId]) -> bool {
    let Some(&last_id) = stmts.last() else {
        return false;
    };
    if let HirStmt::Expr { expr, .. } = &hir.stmts[last_id] {
        matches!(
            hir.exprs[*expr],
            HirExpr::Return { .. } | HirExpr::Break { .. } | HirExpr::Continue { .. }
        )
    } else {
        false
    }
}

/// Generate constraints for a closure expression.
fn gen_closure(
    ctx: &mut InferCtx<'_>,
    hir: &HirBody,
    params: &[HirClosureParam],
    body: &HirBlock,
) -> TyVar {
    // Fresh TyVars for each param (may have type annotation)
    let param_tvs: Vec<TyVar> = params
        .iter()
        .map(|p| {
            let tv = if let Some(ty) = &p.ty {
                lower_hir_ty(ctx, ty)
            } else {
                ctx.fresh()
            };
            ctx.local_types.insert(p.local, tv);
            tv
        })
        .collect();

    // Infer body
    let body_tv = gen_block(ctx, hir, body);

    // Build function type and track closure flexibility
    let fn_tv = ctx.function(param_tvs, body_tv);

    if params.is_empty() {
        // No explicit params, no `it` — adapts to any expected arity
        ctx.closure_flex.insert(fn_tv);
    } else if params.len() == 1 && hir.locals[params[0].local].name == "it" {
        // Implicit `it` — requires exactly 1-param context
        ctx.closure_it.insert(fn_tv);
    }

    fn_tv
}

/// Instantiate an entity's type: reads the ECS to determine what kind of
/// entity it is, creates fresh TyVars for type params, and returns the
/// appropriate type (Function for callables, Named for types, etc.).
/// Instantiate an entity, using explicit type args if provided.
/// Instantiate a generic entity, returning (entity_type, fresh_type_arg_vars).
/// The type arg vars can be recorded in ctx.type_args for later retrieval.
/// True when `protocol` is the enclosing Self for the current body —
/// i.e. some ancestor of `ctx.owner` is either `protocol` itself or an
/// `Extension` whose target resolves to `protocol`.
///
/// Used to lower `Def(Protocol)` values as `SelfType` rather than `Named`,
/// so MIR sees `MirTy::SelfType` and monomorphization substitutes the
/// concrete conforming type at the call site.
fn is_enclosing_self_protocol(ctx: &InferCtx<'_>, protocol: Entity) -> bool {
    let qctx = ctx.query_ctx;
    let mut current = Some(ctx.owner);
    while let Some(entity) = current {
        match qctx.get::<NodeKind>(entity) {
            Some(&NodeKind::Protocol) if entity == protocol => return true,
            Some(&NodeKind::Extension) => {
                if qctx
                    .query(kestrel_name_res::ExtensionTargetEntity {
                        extension: entity,
                        root: ctx.root,
                    })
                    == Some(protocol)
                {
                    return true;
                }
            },
            // Struct/Enum/Module/etc. — Self refers to those, not the protocol.
            Some(&NodeKind::Struct | &NodeKind::Enum) => return false,
            _ => {},
        }
        current = qctx.parent_of(entity);
    }
    false
}

fn instantiate_entity_with_args(
    ctx: &mut InferCtx<'_>,
    entity: Entity,
    explicit_type_args: &[HirTy],
    site_span: &Span,
) -> (TyVar, Vec<TyVar>) {
    instantiate_entity_inner(ctx, entity, explicit_type_args, site_span)
}

fn instantiate_entity_inner(
    ctx: &mut InferCtx<'_>,
    entity: Entity,
    explicit_type_args: &[HirTy],
    site_span: &Span,
) -> (TyVar, Vec<TyVar>) {
    let qctx = ctx.query_ctx;
    let root = ctx.root;

    // Read type params and create fresh TyVars for them,
    // or use explicit type args if provided (e.g., Pointer[UInt8]).
    let type_param_entities: Vec<Entity> = qctx
        .get::<TypeParams>(entity)
        .map(|tp| tp.0.clone())
        .unwrap_or_default();

    // Validate explicit type arg arity
    if !explicit_type_args.is_empty() && explicit_type_args.len() != type_param_entities.len() {
        // Check parent entity fallback (e.g., Pointer[UInt8].nullPointer)
        // Also check extension target for extension methods (e.g., Box[i64].zero)
        let parent = qctx.parent_of(entity);
        let parent_matches = parent
            .and_then(|p| qctx.get::<TypeParams>(p))
            .is_some_and(|tp| tp.0.len() == explicit_type_args.len());
        let ext_target_matches = parent
            .filter(|p| qctx.get::<NodeKind>(*p) == Some(&NodeKind::Extension))
            .and_then(|ext| {
                qctx.query(kestrel_name_res::ExtensionTargetEntity {
                    extension: ext,
                    root: ctx.root,
                })
            })
            .and_then(|target| qctx.get::<TypeParams>(target))
            .is_some_and(|tp| tp.0.len() == explicit_type_args.len());
        if !parent_matches && !ext_target_matches {
            let total = type_param_entities.len();
            let span = hir_ty_span(&explicit_type_args[0]);
            ctx.report_error(InferError::TypeArgCountMismatch {
                expected: total,
                got: explicit_type_args.len(),
                span,
            });
        }
    }

    let fresh_type_args: Vec<TyVar> = if !explicit_type_args.is_empty()
        && explicit_type_args.len() == type_param_entities.len()
    {
        explicit_type_args
            .iter()
            .map(|t| lower_hir_ty(ctx, t))
            .collect()
    } else {
        type_param_entities.iter().map(|_| ctx.fresh()).collect()
    };

    // Build substitution map: type param entity → fresh TyVar.
    // Used to instantiate generic param/return types with fresh vars.
    let mut subs: Vec<(Entity, TyVar)> = type_param_entities
        .iter()
        .zip(fresh_type_args.iter())
        .map(|(&e, &tv)| (e, tv))
        .collect();

    // If explicit type args don't match this entity's params (e.g., Pointer[UInt8].nullPointer
    // where [UInt8] is for Pointer's T, not nullPointer's params), check the parent entity.
    // Also handles extension methods: Box[i64].wrap where [i64] maps to Box's T.
    if !explicit_type_args.is_empty() && explicit_type_args.len() != type_param_entities.len()
        && let Some(parent) = qctx.parent_of(entity) {
            let parent_type_params: Vec<Entity> = qctx
                .get::<TypeParams>(parent)
                .map(|tp| tp.0.clone())
                .unwrap_or_default();
            if explicit_type_args.len() == parent_type_params.len() {
                for (i, &param) in parent_type_params.iter().enumerate() {
                    let tv = lower_hir_ty(ctx, &explicit_type_args[i]);
                    subs.push((param, tv));
                }
            } else if qctx.get::<NodeKind>(parent) == Some(&NodeKind::Extension) {
                // Extension method: map explicit type args to the extension target's type params
                if let Some(target) = qctx.query(kestrel_name_res::ExtensionTargetEntity {
                    extension: parent,
                    root,
                }) {
                    let target_type_params: Vec<Entity> = qctx
                        .get::<TypeParams>(target)
                        .map(|tp| tp.0.clone())
                        .unwrap_or_default();
                    if explicit_type_args.len() == target_type_params.len() {
                        for (i, &param) in target_type_params.iter().enumerate() {
                            let tv = lower_hir_ty(ctx, &explicit_type_args[i]);
                            subs.push((param, tv));
                        }
                    }
                }
            }
        }

    // For methods/functions inside extensions, the extension target's type params
    // need fresh TyVars even without explicit type args. Without this, return types
    // like `Box[T]` stay unresolved when calling `Box.wrap(42)` directly.
    if explicit_type_args.is_empty() && type_param_entities.is_empty()
        && let Some(parent) = qctx.parent_of(entity)
            && qctx.get::<NodeKind>(parent) == Some(&NodeKind::Extension) {
                // Get the extension target's type params (e.g., Box's T)
                if let Some(target) = qctx.query(kestrel_name_res::ExtensionTargetEntity {
                    extension: parent,
                    root,
                }) {
                    let target_type_params: Vec<Entity> = qctx
                        .get::<TypeParams>(target)
                        .map(|tp| tp.0.clone())
                        .unwrap_or_default();
                    for &param in &target_type_params {
                        if !subs.iter().any(|(e, _)| *e == param) {
                            let fresh = ctx.fresh();
                            subs.push((param, fresh));
                        }
                    }
                }
            }

    emit_where_clause_constraints_with_subs(ctx, entity, &subs, site_span);

    // Determine entity kind and build appropriate type
    let type_arg_vars = fresh_type_args.clone();
    let kind = qctx.get::<NodeKind>(entity);
    let entity_tv = match kind {
        // Functions and initializers: build Function type from Callable + return type
        Some(NodeKind::Function | NodeKind::Initializer) => {
            if let Some(param_hir_tys) = qctx.query(LowerCallableTypes { entity, root }) {
                let param_tvs: Vec<TyVar> = param_hir_tys
                    .iter()
                    .map(|t| match t {
                        Some(hir_ty) => lower_hir_ty_with_subs(ctx, hir_ty, &subs),
                        None => ctx.fresh(),
                    })
                    .collect();

                let ret_hir = qctx.query(LowerCallableReturnType { entity, root });
                let ret_tv = lower_hir_ty_with_subs(ctx, &ret_hir, &subs);

                ctx.function(param_tvs, ret_tv)
            } else {
                // Callable without params (shouldn't happen for functions)
                ctx.named(entity, fresh_type_args)
            }
        },

        // Enum case: build function type (payload → parent enum) or unit
        Some(NodeKind::EnumCase) => {
            let parent_enum = qctx.parent_of(entity);

            // For enum cases, type params come from the parent enum.
            // Reuse entries already pushed into the outer `subs` map (e.g. when
            // the caller wrote `Option[Int].None` — the parent's T was mapped to
            // i64 at line 1142-1146). Only allocate fresh TyVars for parent type
            // params that aren't already bound.
            let parent_subs: Vec<(Entity, TyVar)> = if let Some(pe) = parent_enum {
                let parent_tps: Vec<Entity> = qctx
                    .get::<TypeParams>(pe)
                    .map(|tp| tp.0.clone())
                    .unwrap_or_default();
                parent_tps
                    .into_iter()
                    .map(|tp| {
                        if let Some(&(_, tv)) = subs.iter().find(|(e, _)| *e == tp) {
                            (tp, tv)
                        } else {
                            (tp, ctx.fresh())
                        }
                    })
                    .collect()
            } else {
                vec![]
            };

            if let Some(param_hir_tys) = qctx.query(LowerCallableTypes { entity, root }) {
                // Case with payload → function from payload to enum type
                let param_tvs: Vec<TyVar> = param_hir_tys
                    .iter()
                    .map(|t| match t {
                        Some(hir_ty) => lower_hir_ty_with_subs(ctx, hir_ty, &parent_subs),
                        None => ctx.fresh(),
                    })
                    .collect();

                // Return type is the parent enum with the same fresh args
                let ret_tv = if let Some(pe) = parent_enum {
                    let parent_args: Vec<TyVar> = parent_subs.iter().map(|&(_, tv)| tv).collect();
                    ctx.named(pe, parent_args)
                } else {
                    ctx.fresh()
                };

                ctx.function(param_tvs, ret_tv)
            } else {
                // Unit case (no payload) → the parent enum type
                if let Some(pe) = parent_enum {
                    let parent_args: Vec<TyVar> = parent_subs.iter().map(|&(_, tv)| tv).collect();
                    ctx.named(pe, parent_args)
                } else {
                    ctx.named(entity, vec![])
                }
            }
        },

        // Types (struct, enum, protocol): return Named type with fresh args.
        // Protocols reached as values (`Self()`, `Self.method()`) from inside
        // the protocol's own body or an extension on it must surface as
        // `SelfType` so monomorphization substitutes the concrete conformer.
        Some(NodeKind::Protocol) if is_enclosing_self_protocol(ctx, entity) => {
            ctx.self_type_ty(entity)
        },
        Some(NodeKind::Struct | NodeKind::Enum | NodeKind::Protocol) => {
            ctx.named(entity, fresh_type_args)
        },

        // Fields: return the field's type
        Some(NodeKind::Field) => qctx
            .query(LowerTypeAnnotation { entity, root })
            .map(|hir_ty| lower_hir_ty(ctx, &hir_ty))
            .unwrap_or_else(|| ctx.fresh()),

        // Type parameters: return Param type, not Named
        Some(NodeKind::TypeParameter) => ctx.param(entity),

        // Default: Named type wrapping the entity
        _ => ctx.named(entity, fresh_type_args),
    };
    (entity_tv, type_arg_vars)
}

fn emit_where_clause_constraints_with_subs(
    ctx: &mut InferCtx<'_>,
    entity: Entity,
    subs: &[(Entity, TyVar)],
    site_span: &Span,
) {
    let where_clauses = ctx.query_ctx.query(crate::where_clauses::WhereClausesOf {
        entity,
        root: ctx.root,
    });
    for clause in where_clauses {
        match clause {
            crate::resolve::WhereClause::Bound {
                param,
                protocol,
                protocol_type_args,
            } => {
                if let Some(&(_, tv)) = subs.iter().find(|(entity, _)| *entity == param) {
                    ctx.conforms(tv, protocol, site_span.clone());
                    let arg_tvs: Vec<TyVar> = protocol_type_args
                        .iter()
                        .map(|hir_ty| lower_hir_ty_with_subs(ctx, hir_ty, subs))
                        .collect();
                    if !arg_tvs.is_empty() {
                        ctx.record_witness_args(tv, protocol, arg_tvs);
                    }
                }
            },
            crate::resolve::WhereClause::TypeEquality {
                param,
                assoc_name,
                rhs,
            } => {
                if let Some(&(_, tv)) = subs.iter().find(|(entity, _)| *entity == param) {
                    let assoc_result = ctx.fresh();
                    ctx.associated(tv, &assoc_name, assoc_result, site_span.clone());
                    let rhs_tv = lower_hir_ty_with_subs(ctx, &rhs, subs);
                    ctx.equal(assoc_result, rhs_tv, site_span.clone());
                }
            },
            crate::resolve::WhereClause::DirectEquality { param, rhs } => {
                if let Some(&(_, tv)) = subs.iter().find(|(entity, _)| *entity == param) {
                    let rhs_tv = lower_hir_ty_with_subs(ctx, &rhs, subs);
                    ctx.types[tv.0 as usize] = crate::ty::TySlot::Redirect(rhs_tv);
                }
            },
        }
    }
}

/// Convert an HirTy (already resolved during HIR lowering) to a TyVar.
pub fn lower_hir_ty(ctx: &mut InferCtx<'_>, ty: &HirTy) -> TyVar {
    lower_hir_ty_with_subs(ctx, ty, &[])
}

/// If `tv` resolves to `Array[E]`, return the TyVar for `E`; otherwise None.
/// Used by `HirStmt::Let` to feed the annotation's element type into an
/// array-literal RHS so element mismatches compare against the target
/// element type rather than each other.
fn extract_array_elem_hint(ctx: &InferCtx<'_>, tv: TyVar) -> Option<TyVar> {
    let resolved = ctx.resolve(tv);
    let TySlot::Resolved(crate::ty::TyKind::Struct { entity, args }) = ctx.slot(resolved) else {
        return None;
    };
    if ctx
        .resolver
        .builtin(kestrel_hir::Builtin::DefaultArrayLiteralType)
        != Some(*entity)
    {
        return None;
    }
    args.first().copied()
}

/// If `tv` resolves to `Dictionary[K, V, ...]`, return TyVars for `(K, V)`.
/// Used by `HirStmt::Let` to feed annotated key/value types into a dictionary
/// literal RHS before entry constraints run.
fn extract_dict_entry_hint(ctx: &InferCtx<'_>, tv: TyVar) -> Option<(TyVar, TyVar)> {
    let resolved = ctx.resolve(tv);
    let TySlot::Resolved(crate::ty::TyKind::Struct { entity, args }) = ctx.slot(resolved) else {
        return None;
    };
    if ctx
        .resolver
        .builtin(kestrel_hir::Builtin::DefaultDictionaryLiteralType)
        != Some(*entity)
    {
        return None;
    }
    Some((*args.first()?, *args.get(1)?))
}

/// If a dictionary entry literal is already known not to be accepted by the
/// expected key/value type, emit the literal-protocol conformance obligation
/// directly and let the bad entry be covered by that diagnostic. Returning
/// true tells the caller to skip the structural equality that would otherwise
/// default the literal and cascade into "expected T got U".
fn emit_dict_literal_acceptance_error(
    ctx: &mut InferCtx<'_>,
    hir: &HirBody,
    expected: TyVar,
    expr: HirExprId,
    span: Span,
) -> bool {
    let Some(literal) = literal_kind_for_expr(hir, expr) else {
        return false;
    };
    let resolved = ctx.resolve(expected);
    let TySlot::Resolved(kind) = ctx.slot(resolved) else {
        return false;
    };
    if crate::unify::conforms_to_literal_protocol(ctx, kind, literal) {
        return false;
    }
    let Some(protocol) = literal_protocol(ctx, literal) else {
        return false;
    };
    ctx.conforms(expected, protocol, span);
    true
}

fn literal_kind_for_expr(hir: &HirBody, expr: HirExprId) -> Option<LiteralKind> {
    let HirExpr::Literal { value, .. } = &hir.exprs[expr] else {
        return None;
    };
    Some(match value {
        HirLiteral::Integer(_) => LiteralKind::Integer,
        HirLiteral::Float(_) => LiteralKind::Float,
        HirLiteral::String { .. } => LiteralKind::String,
        HirLiteral::Char(_) => LiteralKind::Char,
        HirLiteral::Bool(_) => LiteralKind::Bool,
        HirLiteral::Null => LiteralKind::Null,
    })
}

fn literal_protocol(ctx: &InferCtx<'_>, literal: LiteralKind) -> Option<Entity> {
    let builtin = match literal {
        LiteralKind::Integer => Builtin::ExpressibleByIntegerLiteral,
        LiteralKind::Float => Builtin::ExpressibleByFloatLiteral,
        LiteralKind::String => Builtin::ExpressibleByStringLiteral,
        LiteralKind::Bool => Builtin::ExpressibleByBoolLiteral,
        LiteralKind::Char => Builtin::ExpressibleByCharLiteral,
        LiteralKind::Null => Builtin::ExpressibleByNullLiteral,
        LiteralKind::Array => Builtin::InternalExpressibleByArrayLiteral,
        LiteralKind::Dictionary => Builtin::InternalExpressibleByDictionaryLiteral,
    };
    ctx.resolver.builtin(builtin)
}

/// Convert HirTy to TyVar, substituting type params found in `subs`.
/// Used when instantiating generic entities: type params become fresh TyVars.
/// Also substitutes associated-type entities (where-clause equalities) in subs.
pub(crate) fn lower_hir_ty_with_subs(
    ctx: &mut InferCtx<'_>,
    ty: &HirTy,
    subs: &[(Entity, TyVar)],
) -> TyVar {
    match ty {
        HirTy::Struct { entity, args, .. } => {
            let arg_tvs: Vec<TyVar> = args
                .iter()
                .map(|a| lower_hir_ty_with_subs(ctx, a, subs))
                .collect();
            ctx.struct_ty(*entity, arg_tvs)
        },
        HirTy::Enum { entity, args, .. } => {
            let arg_tvs: Vec<TyVar> = args
                .iter()
                .map(|a| lower_hir_ty_with_subs(ctx, a, subs))
                .collect();
            ctx.enum_ty(*entity, arg_tvs)
        },
        HirTy::Protocol { entity, args, .. } => {
            let arg_tvs: Vec<TyVar> = args
                .iter()
                .map(|a| lower_hir_ty_with_subs(ctx, a, subs))
                .collect();
            ctx.protocol_ty(*entity, arg_tvs)
        },
        HirTy::AliasUse { entity, args, .. } => {
            // Zero-arg alias uses participate in associated-type substitution:
            // where clauses map specific TypeAlias entities to TyVars.
            if args.is_empty() {
                if let Some(&(_, tv)) = subs.iter().find(|(e, _)| e == entity) {
                    return tv;
                }
                if let Some(&(_, tv)) = ctx
                    .where_clause_assoc_subs
                    .iter()
                    .find(|(e, _)| e == entity)
                {
                    return tv;
                }
                if let Some(&tv) = ctx.param_tyvars.get(entity) {
                    return tv;
                }
            }
            let arg_tvs: Vec<TyVar> = args
                .iter()
                .map(|a| lower_hir_ty_with_subs(ctx, a, subs))
                .collect();
            ctx.type_alias(*entity, arg_tvs)
        },
        HirTy::AssocProjection { base, assoc, span } => {
            let base_tv = lower_hir_ty_with_subs(ctx, base, subs);
            // Short-circuit if this specific assoc is already bound via a
            // where-clause equality (e.g. `Item = (A, B)`).
            if let Some(&(_, tv)) = ctx.where_clause_assoc_subs.iter().find(|(e, _)| e == assoc) {
                let _ = base_tv;
                return tv;
            }
            ctx.project_associated(base_tv, *assoc, span.clone())
        },
        HirTy::Tuple(types, _) => {
            let elem_tvs: Vec<TyVar> = types
                .iter()
                .map(|t| lower_hir_ty_with_subs(ctx, t, subs))
                .collect();
            ctx.tuple(elem_tvs)
        },
        HirTy::Function { params, ret, .. } => {
            let param_tvs: Vec<TyVar> = params
                .iter()
                .map(|p| lower_hir_ty_with_subs(ctx, p, subs))
                .collect();
            let ret_tv = lower_hir_ty_with_subs(ctx, ret, subs);
            ctx.function(param_tvs, ret_tv)
        },
        HirTy::Param(entity, _) => {
            // Check substitution map first (for instantiated type params)
            if let Some(&(_, tv)) = subs.iter().find(|(e, _)| e == entity) {
                return tv;
            }
            ctx.param(*entity)
        },
        HirTy::SelfType(entity, _) => {
            // Preserve the "this is Self" identity through inference output so
            // MIR receives `MirTy::SelfType` and monomorphization substitutes
            // it with the caller's concrete self type. `TyKind::SelfType(P)`
            // behaves like `Protocol(P)` for associated-type / conformance
            // lookups but is distinguished at `kind_to_resolved`.
            // `lower_hir_ty_sub` still substitutes this with the concrete
            // receiver TyVar at method-dispatch time.
            ctx.self_type_ty(*entity)
        },
        HirTy::Never(_) => ctx.never(),
        HirTy::Infer(_) => {
            let tv = ctx.fresh();
            ctx.mark_wildcard(tv);
            tv
        },
        HirTy::Error(span) => ctx.report_error(InferError::FromHir { span: span.clone() }),
    }
}

/// Convert a literal value to a TyVar with a literal kind marker.
fn literal_to_tyvar(ctx: &mut InferCtx<'_>, value: &HirLiteral) -> TyVar {
    match value {
        HirLiteral::Integer(_) => ctx.fresh_literal(LiteralKind::Integer),
        HirLiteral::Float(_) => ctx.fresh_literal(LiteralKind::Float),
        HirLiteral::String { .. } => ctx.fresh_literal(LiteralKind::String),
        HirLiteral::Char(_) => ctx.fresh_literal(LiteralKind::Char),
        HirLiteral::Bool(_) => ctx.fresh_literal(LiteralKind::Bool),
        HirLiteral::Null => ctx.fresh_literal(LiteralKind::Null),
    }
}

/// Extract the span from an expression.
/// Whether this If expression is the desugared form of a guard statement.
/// Guard lowers to `HirStmt::Expr { HirExpr::If { then: empty, else: body } }`
/// and the statement id is tracked in `hir.guard_stmts`.
fn is_guard_if(hir: &HirBody, expr_id: HirExprId) -> bool {
    hir.guard_stmts.iter().any(
        |&stmt_id| matches!(&hir.stmts[stmt_id], HirStmt::Expr { expr, .. } if *expr == expr_id),
    )
}

/// Get the span of a block's value expression (tail expr or last statement expr).
/// Returns None if the block has no value expression.
fn block_value_span(hir: &HirBody, block: &HirBlock) -> Option<Span> {
    if let Some(tail) = block.tail_expr {
        return Some(expr_span(hir, tail));
    }
    // Check if last statement is an expression
    if let Some(&last_id) = block.stmts.last()
        && let HirStmt::Expr { expr, .. } = &hir.stmts[last_id] {
            return Some(expr_span(hir, *expr));
        }
    None
}

fn expr_span(hir: &HirBody, id: HirExprId) -> Span {
    match &hir.exprs[id] {
        HirExpr::Literal { span, .. }
        | HirExpr::Tuple { span, .. }
        | HirExpr::Array { span, .. }
        | HirExpr::Dict { span, .. }
        | HirExpr::Closure { span, .. }
        | HirExpr::Local(_, span)
        | HirExpr::Def(_, _, span)
        | HirExpr::OverloadSet { span, .. }
        | HirExpr::Field { span, .. }
        | HirExpr::TupleIndex { span, .. }
        | HirExpr::ImplicitMember { span, .. }
        | HirExpr::Call { span, .. }
        | HirExpr::MethodCall { span, .. }
        | HirExpr::ProtocolCall { span, .. }
        | HirExpr::If { span, .. }
        | HirExpr::Loop { span, .. }
        | HirExpr::Match { span, .. }
        | HirExpr::Break { span, .. }
        | HirExpr::Continue { span, .. }
        | HirExpr::Return { span, .. }
        | HirExpr::Assign { span, .. }
        | HirExpr::Block { span, .. }
        | HirExpr::Error { span }
        | HirExpr::Sugar { span, .. } => span.clone(),
    }
}

/// Extract a span from a HirTy.
fn hir_ty_span(ty: &HirTy) -> Span {
    match ty {
        HirTy::Struct { span, .. }
        | HirTy::Enum { span, .. }
        | HirTy::Protocol { span, .. }
        | HirTy::AliasUse { span, .. }
        | HirTy::AssocProjection { span, .. }
        | HirTy::Tuple(_, span)
        | HirTy::Function { span, .. }
        | HirTy::Param(_, span)
        | HirTy::SelfType(_, span)
        | HirTy::Never(span)
        | HirTy::Infer(span)
        | HirTy::Error(span) => span.clone(),
    }
}

/// Generate constraints for a fully-resolved enum variant pattern.
/// Constrains scrutinee to be the parent enum type and binds arg patterns
/// to the case's payload types.
fn gen_variant_pat(
    ctx: &mut InferCtx<'_>,
    hir: &HirBody,
    entity: Entity,
    args: &[HirPatArg],
    scrutinee_tv: TyVar,
    span: &Span,
    source: MatchSource,
) {
    let qctx = ctx.query_ctx;
    let root = ctx.root;

    // Build substitution map: enum type params → fresh TyVars linked to scrutinee.
    // This ensures payload types like HirTy::Param(T) resolve to the scrutinee's
    // actual type arg, not the canonical Param TyVar.
    let mut subs: Vec<(Entity, TyVar)> = Vec::new();
    if let Some(parent_enum) = qctx.parent_of(entity) {
        let parent_tps: Vec<Entity> = qctx
            .get::<TypeParams>(parent_enum)
            .map(|tp| tp.0.clone())
            .unwrap_or_default();
        let parent_args: Vec<TyVar> = parent_tps.iter().map(|_| ctx.fresh()).collect();
        let enum_tv = ctx.named(parent_enum, parent_args.clone());

        // Scrutinee must be this enum type
        ctx.equal(scrutinee_tv, enum_tv, span.clone());

        // Map enum type params to the fresh args linked to scrutinee
        for (&param, &arg) in parent_tps.iter().zip(parent_args.iter()) {
            subs.push((param, arg));
        }
    }

    // Get case's payload types, substituting enum type params with scrutinee args
    let payload_types: Vec<TyVar> = match qctx.query(LowerCallableTypes { entity, root }) {
        Some(types) => types
            .iter()
            .map(|t| match t {
                Some(hir_ty) => lower_hir_ty_with_subs(ctx, hir_ty, &subs),
                None => ctx.fresh(),
            })
            .collect(),
        // No Callable → unit case, args should be empty
        None => vec![],
    };

    // Constrain each arg pattern against the corresponding payload type
    let payload_len = payload_types.len();
    for (arg, payload_tv) in args.iter().zip(payload_types) {
        gen_pat(ctx, hir, arg.pattern, payload_tv, source);
    }
    // Extra args with no corresponding type get fresh TyVars
    for arg in args.iter().skip(payload_len) {
        let arg_tv = ctx.fresh();
        gen_pat(ctx, hir, arg.pattern, arg_tv, source);
    }
}

/// Generate constraints for an implicit variant pattern (.CaseName).
/// Creates fresh TyVars for each sub-pattern binding and emits an
/// ImplicitPat constraint. The solver defers until the scrutinee type
/// is concrete, then looks up the case and equates the binding TyVars
/// with the payload types (properly substituted with scrutinee type args).
fn gen_implicit_variant_pat(
    ctx: &mut InferCtx<'_>,
    hir: &HirBody,
    name: &str,
    args: &[HirPatArg],
    scrutinee_tv: TyVar,
    span: &Span,
    source: MatchSource,
) {
    let arg_tys: Vec<TyVar> = args
        .iter()
        .map(|arg| {
            let tv = ctx.fresh();
            gen_pat(ctx, hir, arg.pattern, tv, source);
            tv
        })
        .collect();

    ctx.constraints
        .push(crate::constraint::Constraint::ImplicitPat {
            scrutinee: scrutinee_tv,
            name: name.to_string(),
            arg_tys,
            span: span.clone(),
        });
}

/// Generate constraints for a struct pattern.
/// Constrains scrutinee to be the struct type and binds each field pattern
/// to the field's declared type.
fn gen_struct_pat(
    ctx: &mut InferCtx<'_>,
    hir: &HirBody,
    entity: Entity,
    fields: &[HirStructPatField],
    scrutinee_tv: TyVar,
    span: &Span,
    source: MatchSource,
) {
    let qctx = ctx.query_ctx;
    let root = ctx.root;

    // Create the struct type with fresh type args
    let type_params: Vec<Entity> = qctx
        .get::<TypeParams>(entity)
        .map(|tp| tp.0.clone())
        .unwrap_or_default();
    let fresh_args: Vec<TyVar> = type_params.iter().map(|_| ctx.fresh()).collect();
    let struct_tv = ctx.named(entity, fresh_args);

    // Scrutinee must be this struct type
    ctx.equal(scrutinee_tv, struct_tv, span.clone());

    // For each field in the pattern, find the matching Field child and constrain
    let children: Vec<Entity> = qctx.children_of(entity).to_vec();
    for field in fields {
        let Some(field_name_str) = field.field_name.as_str() else {
            // Pattern field with missing name — parser already reported the
            // gap; skip resolution so we don't emit a "no field" cascade.
            continue;
        };
        let found = children.iter().find(|&&child| {
            qctx.get::<NodeKind>(child) == Some(&NodeKind::Field)
                && qctx
                    .get::<Name>(child)
                    .is_some_and(|n| n.0 == field_name_str)
        });
        let field_tv = found
            .and_then(|&child| {
                qctx.query(LowerTypeAnnotation {
                    entity: child,
                    root,
                })
                .map(|hir_ty| lower_hir_ty(ctx, &hir_ty))
            })
            .unwrap_or_else(|| {
                let tv = ctx.fresh();
                // Unknown field — an analyzer reports the error. Poison the
                // binding TyVar to suppress a cascading "could not infer type".
                if found.is_none() {
                    ctx.poison(tv);
                }
                tv
            });

        if let Some(pat) = field.pattern {
            gen_pat(ctx, hir, pat, field_tv, source);
        }
    }
}
