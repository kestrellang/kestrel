//! Constraint generation: walk HirBody once, emitting constraints for every node.
//!
//! This is a one-pass walk that creates TyVars for every expression,
//! statement binding, and local variable, then emits constraints
//! capturing the type relationships between them.

use kestrel_ast_builder::{Callable, Name, NodeKind, TypeParams};
use kestrel_hecs::Entity;
use kestrel_hir::body::*;
use kestrel_hir::ty::HirTy;
use kestrel_hir_lower::{LowerCallableTypes, LowerTypeAnnotation};
use kestrel_span2::Span;

use crate::constraint::{labels_match, CallArg, Constraint};
use crate::ctx::InferCtx;
use crate::error::InferError;
use crate::ty::{LiteralKind, TyVar};

// ===== Entry point =====

/// Generate constraints for an entire HirBody.
/// Creates TyVars for params and return type, walks all statements and tail expr.
pub fn generate(
    ctx: &mut InferCtx<'_>,
    hir: &HirBody,
    param_types: &[TyVar],
    return_ty: TyVar,
) {
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

    // Tail expression flows to return type
    if let Some(tail) = hir.tail_expr {
        let tail_tv = gen_expr(ctx, hir, tail);
        let span = expr_span(hir, tail);
        ctx.coerce(tail_tv, ctx.return_ty, tail, span);
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
            HirLiteral::String(_) => ctx.fresh_literal(LiteralKind::String),
            HirLiteral::Char(_) => ctx.fresh_literal(LiteralKind::Char),
            HirLiteral::Bool(_) => ctx.fresh_literal(LiteralKind::Bool),
            HirLiteral::Null => ctx.fresh_literal(LiteralKind::Null),
        },

        // === References ===
        HirExpr::Local(local_id, _) => {
            // Return the TyVar assigned when this local was declared
            ctx.local_types.get(local_id).copied().unwrap_or_else(|| {
                let local = &hir.locals[*local_id];
                kestrel_debug::ktrace!("hir-lower", "missing local type for {:?} (local_id {:?})",
                    local.name, local_id);
                ctx.report_error(InferError::FromHir {
                    span: expr_span(hir, id),
                })
            })
        }

        HirExpr::Def(entity, explicit_type_args, span) => {
            // Read the entity's type from the world via the resolver.
            // For generic entities, this will instantiate fresh TyVars.
            // If explicit type args are provided (e.g., Pointer[UInt8]),
            // use them instead of fresh inference variables.
            let (tv, type_arg_vars) = instantiate_entity_with_args(ctx, *entity, explicit_type_args);
            // Record type arg vars so MIR lowering can retrieve the resolved types
            if !type_arg_vars.is_empty() {
                ctx.type_args.insert(id, type_arg_vars);
            }
            // Track type parameter references — invalid unless consumed by MethodCall/Call
            if ctx.query_ctx.get::<NodeKind>(*entity) == Some(&NodeKind::TypeParameter) {
                ctx.type_param_defs.insert(id, span.clone());
            }
            tv
        }

        // Overloaded function reference — can only appear as callee of Call
        HirExpr::OverloadSet { span, .. } => {
            let recv = ctx.fresh();
            ctx.report_error(InferError::AmbiguousMember {
                receiver: recv,
                name: "<overloaded function>".into(),
                span: span.clone(),
            })
        }

        // === Calls ===
        HirExpr::Call { callee, args, span } => {
            // Overloaded free function call: emit OverloadedCall constraint
            if let HirExpr::OverloadSet { candidates, type_args, .. } = &hir.exprs[*callee] {
                let candidates = candidates.clone();
                let type_args = type_args.clone();
                let arg_tvs = gen_call_args(ctx, hir, args);
                let result_tv = ctx.fresh();
                ctx.overloaded_call(candidates, type_args, arg_tvs, result_tv, id, span.clone());
                ctx.expr_types.insert(id, result_tv);
                return result_tv;
            }

            // Struct construction: Def(struct) used as callee
            if let HirExpr::Def(entity, _, _) = &hir.exprs[*callee] {
                if ctx.query_ctx.get::<NodeKind>(*entity) == Some(&NodeKind::Struct) {
                    let arg_tvs = gen_call_args(ctx, hir, args);
                    return gen_struct_init(ctx, *entity, &arg_tvs, id, span);
                }
            }

            // Enum case construction: route through OverloadedCall for label checking
            if let HirExpr::Def(entity, _, _) = &hir.exprs[*callee] {
                if ctx.query_ctx.get::<NodeKind>(*entity) == Some(&NodeKind::EnumCase)
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
                if let Some(HirTy::Never(_)) = ctx.query_ctx.query(
                    kestrel_hir_lower::LowerTypeAnnotation { entity: *entity, root: ctx.root }
                ) {
                    ctx.never()
                } else {
                    ctx.fresh()
                }
            } else {
                ctx.fresh()
            };

            ctx.call(callee_tv, arg_tvs, result_tv, id, span.clone());
            result_tv
        }

        HirExpr::MethodCall {
            receiver,
            method,
            args,
            span,
            ..
        } => {
            let recv_tv = gen_expr(ctx, hir, *receiver);
            let arg_tvs = gen_call_args(ctx, hir, args);
            let result_tv = ctx.fresh();

            // Check if receiver is a type parameter reference (T.method() = static context)
            let is_static_ctx = matches!(
                &hir.exprs[*receiver],
                HirExpr::Def(entity, _, _) if ctx.query_ctx.get::<NodeKind>(*entity)
                    == Some(&NodeKind::TypeParameter)
            );

            // Consuming a Def(TypeParameter) as a MethodCall receiver is valid
            if is_static_ctx {
                ctx.type_param_defs.remove(receiver);
                ctx.member_static(recv_tv, method, arg_tvs, result_tv, id, true, span.clone());
            } else {
                ctx.member(recv_tv, method, arg_tvs, result_tv, id, true, span.clone());
            }
            result_tv
        }

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

            // Receiver must conform to the protocol
            ctx.conforms(recv_tv, *protocol, span.clone());
            // Resolve method on the protocol
            ctx.member(recv_tv, method, arg_tvs, result_tv, id, true, span.clone());
            result_tv
        }

        // === Member access ===
        HirExpr::Field {
            base, name, span, ..
        } => {
            let base_tv = gen_expr(ctx, hir, *base);
            let result_tv = ctx.fresh();

            // Member constraint with no args (field/property access)
            ctx.member(base_tv, name, vec![], result_tv, id, false, span.clone());
            result_tv
        }

        HirExpr::TupleIndex {
            base, index, span, ..
        } => {
            let base_tv = gen_expr(ctx, hir, *base);
            let result_tv = ctx.fresh();

            // Emit an Equal constraint: base must be a tuple, and result = element at index.
            // The solver will extract the element type when base resolves.
            // For now we use a Member constraint with the index as name.
            ctx.member(
                base_tv,
                &index.to_string(),
                vec![],
                result_tv,
                id,
                false,
                span.clone(),
            );
            result_tv
        }

        // === Implicit member (.CaseName) ===
        HirExpr::ImplicitMember { name, args, span } => {
            let arg_tvs = args
                .as_ref()
                .map(|a| gen_call_args(ctx, hir, a))
                .unwrap_or_default();
            let result_tv = ctx.fresh();

            // Implicit constraint: resolved against expected type
            ctx.implicit(result_tv, name, arg_tvs, result_tv, id, span.clone());
            result_tv
        }

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
                let else_span = block_value_span(hir, else_block).unwrap_or_else(|| span.clone());
                ctx.equal(then_tv, result_tv, then_span);
                ctx.equal(else_tv, result_tv, else_span);
                result_tv
            } else {
                // No else: expression has unit type
                ctx.tuple(vec![])
            }
        }

        HirExpr::Match {
            scrutinee,
            arms,
            span,
        } => {
            let scrut_tv = gen_expr(ctx, hir, *scrutinee);
            let result_tv = ctx.fresh();

            for arm in arms {
                // Pattern constrains scrutinee type
                gen_pat(ctx, hir, arm.pattern, scrut_tv);

                // Guard must be bool
                if let Some(guard) = arm.guard {
                    // Guard type validated later (same as if-condition)
                    let _guard_tv = gen_expr(ctx, hir, guard);
                }

                // Body must match result type
                let body_tv = gen_expr(ctx, hir, arm.body);
                ctx.equal(body_tv, result_tv, span.clone());
            }
            result_tv
        }

        HirExpr::Loop { body, .. } => {
            gen_block(ctx, hir, body);
            // Loop type is Never (diverges) unless break provides a value
            ctx.never()
        }

        HirExpr::Break { .. } | HirExpr::Continue { .. } => ctx.never(),

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
        }

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
        }

        // === Closures ===
        HirExpr::Closure { params, body, .. } => gen_closure(ctx, hir, params, body),

        // === Aggregates ===
        HirExpr::Array { elements, span } => {
            let elem_tv = ctx.fresh();
            for &e in elements {
                let e_tv = gen_expr(ctx, hir, e);
                ctx.equal(e_tv, elem_tv, span.clone());
            }
            // Result is an array literal — will default to DefaultArrayLiteralType[elem]
            ctx.fresh_literal(LiteralKind::Array)
        }

        HirExpr::Dict { entries, span } => {
            let key_tv = ctx.fresh();
            let val_tv = ctx.fresh();
            for entry in entries {
                let k = gen_expr(ctx, hir, entry.key);
                let v = gen_expr(ctx, hir, entry.value);
                ctx.equal(k, key_tv, span.clone());
                ctx.equal(v, val_tv, span.clone());
            }
            ctx.fresh_literal(LiteralKind::Dictionary)
        }

        HirExpr::Tuple { elements, span: _ } => {
            let elem_tvs: Vec<TyVar> = elements.iter().map(|&e| gen_expr(ctx, hir, e)).collect();
            ctx.tuple(elem_tvs)
        }

        // Block expression: execute stmts, result is the tail expr
        HirExpr::Block { body, .. } => gen_block(ctx, hir, body),

        HirExpr::Error { span } => ctx.report_error(InferError::FromHir { span: span.clone() }),
    };

    // Record the type for this expression
    ctx.expr_types.insert(id, tv);
    tv
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
                let val_tv = gen_expr(ctx, hir, *val);
                // Value flows to the binding (allows promotion)
                ctx.coerce(val_tv, local_tv, *val, span.clone());
            }
        }

        HirStmt::Expr { expr, .. } => {
            gen_expr(ctx, hir, *expr);
        }

        HirStmt::Deinit { .. } => {
            // Deinit has no type semantics for inference
        }
    }
}

// ===== Pattern generation =====

/// Generate constraints for a pattern given the type of the scrutinee.
fn gen_pat(ctx: &mut InferCtx<'_>, hir: &HirBody, pat_id: HirPatId, scrutinee_tv: TyVar) {
    match &hir.pats[pat_id] {
        HirPat::Wildcard { .. } => {
            // No constraint — matches anything
        }

        HirPat::Binding { local, .. } => {
            // Bind local to the scrutinee type
            ctx.local_types.insert(*local, scrutinee_tv);
        }

        HirPat::Literal { value, span, .. } => {
            let lit_tv = literal_to_tyvar(ctx, value);
            ctx.equal(lit_tv, scrutinee_tv, span.clone());
        }

        HirPat::Tuple { prefix, has_rest, suffix, span } => {
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
                // Fixed arity: equate scrutinee against tuple of exactly these elements
                let tuple_tv = ctx.tuple(prefix_tvs.clone());
                ctx.equal(scrutinee_tv, tuple_tv, span.clone());
            }

            for (&elem_pat, elem_tv) in prefix.iter().zip(prefix_tvs) {
                gen_pat(ctx, hir, elem_pat, elem_tv);
            }
            for (&elem_pat, elem_tv) in suffix.iter().zip(suffix_tvs) {
                gen_pat(ctx, hir, elem_pat, elem_tv);
            }
        }

        HirPat::Variant {
            entity,
            args,
            span,
        } => {
            gen_variant_pat(ctx, hir, *entity, args, scrutinee_tv, span);
        }

        HirPat::ImplicitVariant { name, args, span } => {
            gen_implicit_variant_pat(ctx, hir, name, args, scrutinee_tv, span);
        }

        HirPat::Struct {
            entity,
            fields,
            span,
            ..
        } => {
            gen_struct_pat(ctx, hir, *entity, fields, scrutinee_tv, span);
        }

        HirPat::Or { alternatives, .. } => {
            for &alt in alternatives {
                gen_pat(ctx, hir, alt, scrutinee_tv);
            }
        }

        HirPat::Range { span, .. } => {
            // Range patterns constrain the scrutinee to be the literal type
            // For now, just accept — range pattern types are validated later
            let _ = span;
        }

        HirPat::At { binding, subpattern, .. } => {
            // Bind the whole matched value to the local, then constrain via subpattern
            ctx.local_types.insert(*binding, scrutinee_tv);
            gen_pat(ctx, hir, *subpattern, scrutinee_tv);
        }

        HirPat::Array { prefix, has_rest: _, suffix, span } => {
            // Each element has the same type (Array[T] → T)
            let elem_tv = ctx.fresh();

            // Build Array[elem_tv] and equate with scrutinee
            if let Some(array_entity) = ctx.resolver.builtin(kestrel_hir::Builtin::DefaultArrayLiteralType) {
                let array_tv = ctx.named(array_entity, vec![elem_tv]);
                ctx.equal(scrutinee_tv, array_tv, span.clone());
            }

            // Equate each prefix/suffix element pattern against elem_tv
            for &elem_pat in prefix.iter().chain(suffix.iter()) {
                let pat_tv = ctx.fresh();
                ctx.equal(pat_tv, elem_tv, span.clone());
                gen_pat(ctx, hir, elem_pat, pat_tv);
            }
        }

        HirPat::Error { .. } => { /* swallow */ }
    }
}

// ===== Struct construction =====

/// Generate constraints for a struct constructor call.
/// Finds a matching init (by arity + label pattern) or uses memberwise from fields.
fn gen_struct_init(
    ctx: &mut InferCtx<'_>,
    struct_entity: Entity,
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
            // Record which init was selected so MIR lowering can use the init entity
            ctx.resolutions.insert(expr_id, init);
            // Build subs that includes both struct type params AND init's own type params
            let init_type_params: Vec<Entity> = qctx
                .get::<TypeParams>(init)
                .map(|tp| tp.0.clone())
                .unwrap_or_default();
            let init_fresh: Vec<TyVar> = init_type_params.iter().map(|_| ctx.fresh()).collect();
            let mut init_subs = struct_subs.clone();
            for (&e, &tv) in init_type_params.iter().zip(init_fresh.iter()) {
                init_subs.push((e, tv));
            }

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
            kestrel_debug::ktrace!(
                "type-infer",
                "no matching init for {:?} with labels {:?}",
                qctx.get::<Name>(struct_entity),
                arg_labels
            );
        } else {
            // Multiple label matches — emit Member constraint.
            // resolve_member will detect the ambiguity and try protocol-based
            // resolution, letting the solver disambiguate via type inference.
            let recv_tv = ctx.named(struct_entity, fresh_args.clone());
            let init_result = ctx.fresh(); // init return type (discarded — result is struct type)
            ctx.member(recv_tv, "init", args.to_vec(), init_result, expr_id, true, span.clone());
            ctx.expr_types.insert(expr_id, result_tv);
            return result_tv;
        }
    } else {
        // Memberwise init: match args against field types (in order)
        let fields: Vec<Entity> = children
            .iter()
            .filter(|&&c| qctx.get::<NodeKind>(c) == Some(&NodeKind::Field))
            .copied()
            .collect();

        for (arg, &field) in args.iter().zip(fields.iter()) {
            if let Some(hir_ty) = qctx.query(LowerTypeAnnotation { entity: field, root }) {
                let field_tv = lower_hir_ty_with_subs(ctx, &hir_ty, &struct_subs);
                ctx.coerce(arg.ty, field_tv, expr_id, span.clone());
            }
        }
    }

    // Record this expr's type and return
    ctx.expr_types.insert(expr_id, result_tv);
    result_tv
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

    // Build function type
    ctx.function(param_tvs, body_tv)
}

/// Instantiate an entity's type: reads the ECS to determine what kind of
/// entity it is, creates fresh TyVars for type params, and returns the
/// appropriate type (Function for callables, Named for types, etc.).
/// Instantiate an entity, using explicit type args if provided.
/// Instantiate a generic entity, returning (entity_type, fresh_type_arg_vars).
/// The type arg vars can be recorded in ctx.type_args for later retrieval.
fn instantiate_entity_with_args(
    ctx: &mut InferCtx<'_>,
    entity: Entity,
    explicit_type_args: &[HirTy],
) -> (TyVar, Vec<TyVar>) {
    instantiate_entity_inner(ctx, entity, explicit_type_args)
}

fn instantiate_entity_inner(
    ctx: &mut InferCtx<'_>,
    entity: Entity,
    explicit_type_args: &[HirTy],
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
            .and_then(|ext| qctx.query(kestrel_name_res::ExtensionTargetEntity {
                extension: ext,
                root: ctx.root,
            }))
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
        explicit_type_args.iter().map(|t| lower_hir_ty(ctx, t)).collect()
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
    if !explicit_type_args.is_empty() && explicit_type_args.len() != type_param_entities.len() {
        if let Some(parent) = qctx.parent_of(entity) {
            let parent_type_params: Vec<Entity> = qctx
                .get::<TypeParams>(parent)
                .map(|tp| tp.0.clone())
                .unwrap_or_default();
            if explicit_type_args.len() == parent_type_params.len() {
                for (i, &param) in parent_type_params.iter().enumerate() {
                    let tv = lower_hir_ty(ctx, &explicit_type_args[i]);
                    subs.push((param, tv));
                }
            }
        }
    }

    // Emit where clause constraints for the type params
    for clause in ctx.resolver.where_clauses(entity) {
        match clause {
            crate::resolve::WhereClause::Bound { param, protocol, .. } => {
                if let Some(idx) = type_param_entities.iter().position(|&p| p == param) {
                    let span = Span::synthetic(0);
                    ctx.conforms(fresh_type_args[idx], protocol, span);
                }
            }
            crate::resolve::WhereClause::TypeEquality {
                param,
                assoc_name,
                rhs,
            } => {
                if let Some(idx) = type_param_entities.iter().position(|&p| p == param) {
                    let span = Span::synthetic(0);
                    let assoc_result = ctx.fresh();
                    ctx.associated(
                        fresh_type_args[idx],
                        &assoc_name,
                        assoc_result,
                        span.clone(),
                    );
                    let rhs_tv = lower_hir_ty_with_subs(ctx, &rhs, &subs);
                    ctx.equal(assoc_result, rhs_tv, span);
                }
            }
            crate::resolve::WhereClause::DirectEquality { param, rhs } => {
                // Direct type param equality: V = Array[E]
                // Redirect the param's TyVar to the concrete RHS type.
                if let Some(idx) = type_param_entities.iter().position(|&p| p == param) {
                    let rhs_tv = lower_hir_ty_with_subs(ctx, &rhs, &subs);
                    ctx.types[fresh_type_args[idx].0 as usize] =
                        crate::ty::TySlot::Redirect(rhs_tv);
                }
            }
        }
    }

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

                let ret_tv = qctx
                    .query(LowerTypeAnnotation { entity, root })
                    .map(|hir_ty| lower_hir_ty_with_subs(ctx, &hir_ty, &subs))
                    .unwrap_or_else(|| ctx.fresh());

                ctx.function(param_tvs, ret_tv)
            } else {
                // Callable without params (shouldn't happen for functions)
                ctx.named(entity, fresh_type_args)
            }
        }

        // Enum case: build function type (payload → parent enum) or unit
        Some(NodeKind::EnumCase) => {
            let parent_enum = qctx.parent_of(entity);

            // For enum cases, type params come from the parent enum.
            // Build subs from parent's type params.
            let parent_subs: Vec<(Entity, TyVar)> = if let Some(pe) = parent_enum {
                let parent_tps: Vec<Entity> = qctx
                    .get::<TypeParams>(pe)
                    .map(|tp| tp.0.clone())
                    .unwrap_or_default();
                let parent_args: Vec<TyVar> =
                    parent_tps.iter().map(|_| ctx.fresh()).collect();
                parent_tps.into_iter().zip(parent_args.into_iter()).collect()
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
        }

        // Types (struct, enum, protocol): return Named type with fresh args
        Some(NodeKind::Struct | NodeKind::Enum | NodeKind::Protocol) => {
            ctx.named(entity, fresh_type_args)
        }

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

/// Convert an HirTy (already resolved during HIR lowering) to a TyVar.
pub fn lower_hir_ty(ctx: &mut InferCtx<'_>, ty: &HirTy) -> TyVar {
    lower_hir_ty_with_subs(ctx, ty, &[])
}

/// Convert HirTy to TyVar, substituting type params found in `subs`.
/// Used when instantiating generic entities: type params become fresh TyVars.
/// Also substitutes Named entities (e.g., TypeAlias for associated types) in subs.
pub(crate) fn lower_hir_ty_with_subs(
    ctx: &mut InferCtx<'_>,
    ty: &HirTy,
    subs: &[(Entity, TyVar)],
) -> TyVar {
    match ty {
        HirTy::Named { entity, args, .. } => {
            // Check substitution map (e.g., associated type entities from where clauses)
            if args.is_empty() {
                if let Some(&(_, tv)) = subs.iter().find(|(e, _)| e == entity) {
                    return tv;
                }
                // Check where_clause_assoc_subs for associated types (e.g., Item → Optional[T])
                if let Some(&(_, tv)) = ctx.where_clause_assoc_subs.iter().find(|(e, _)| e == entity) {
                    return tv;
                }
                // Also check param_tyvars for redirected params (from DirectEquality)
                if let Some(&tv) = ctx.param_tyvars.get(entity) {
                    return tv;
                }
            }
            let arg_tvs: Vec<TyVar> = args.iter().map(|a| lower_hir_ty_with_subs(ctx, a, subs)).collect();
            ctx.named(*entity, arg_tvs)
        }
        HirTy::Tuple(types, _) => {
            let elem_tvs: Vec<TyVar> = types.iter().map(|t| lower_hir_ty_with_subs(ctx, t, subs)).collect();
            ctx.tuple(elem_tvs)
        }
        HirTy::Function { params, ret, .. } => {
            let param_tvs: Vec<TyVar> = params.iter().map(|p| lower_hir_ty_with_subs(ctx, p, subs)).collect();
            let ret_tv = lower_hir_ty_with_subs(ctx, ret, subs);
            ctx.function(param_tvs, ret_tv)
        }
        HirTy::Param(entity, _) => {
            // Check substitution map first (for instantiated type params)
            if let Some(&(_, tv)) = subs.iter().find(|(e, _)| e == entity) {
                return tv;
            }
            ctx.param(*entity)
        }
        HirTy::Never(_) => ctx.never(),
        HirTy::Infer(_) => ctx.fresh(),
        HirTy::Error(span) => ctx.report_error(InferError::FromHir { span: span.clone() }),
    }
}

/// Convert a literal value to a TyVar with a literal kind marker.
fn literal_to_tyvar(ctx: &mut InferCtx<'_>, value: &HirLiteral) -> TyVar {
    match value {
        HirLiteral::Integer(_) => ctx.fresh_literal(LiteralKind::Integer),
        HirLiteral::Float(_) => ctx.fresh_literal(LiteralKind::Float),
        HirLiteral::String(_) => ctx.fresh_literal(LiteralKind::String),
        HirLiteral::Char(_) => ctx.fresh_literal(LiteralKind::Char),
        HirLiteral::Bool(_) => ctx.fresh_literal(LiteralKind::Bool),
        HirLiteral::Null => ctx.fresh_literal(LiteralKind::Null),
    }
}

/// Extract the span from an expression.
/// Get the span of a block's value expression (tail expr or last statement expr).
/// Returns None if the block has no value expression.
fn block_value_span(hir: &HirBody, block: &HirBlock) -> Option<Span> {
    if let Some(tail) = block.tail_expr {
        return Some(expr_span(hir, tail));
    }
    // Check if last statement is an expression
    if let Some(&last_id) = block.stmts.last() {
        if let HirStmt::Expr { expr, .. } = &hir.stmts[last_id] {
            return Some(expr_span(hir, *expr));
        }
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
        | HirExpr::Error { span } => span.clone(),
    }
}

/// Extract a span from a HirTy.
fn hir_ty_span(ty: &HirTy) -> Span {
    match ty {
        HirTy::Named { span, .. }
        | HirTy::Tuple(_, span)
        | HirTy::Function { span, .. }
        | HirTy::Param(_, span)
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
    let payload_types: Vec<TyVar> =
        match qctx.query(LowerCallableTypes { entity, root }) {
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
        gen_pat(ctx, hir, arg.pattern, payload_tv);
    }
    // Extra args with no corresponding type get fresh TyVars
    for arg in args.iter().skip(payload_len) {
        let arg_tv = ctx.fresh();
        gen_pat(ctx, hir, arg.pattern, arg_tv);
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
) {
    let arg_tys: Vec<TyVar> = args
        .iter()
        .map(|arg| {
            let tv = ctx.fresh();
            gen_pat(ctx, hir, arg.pattern, tv);
            tv
        })
        .collect();

    ctx.constraints.push(crate::constraint::Constraint::ImplicitPat {
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
        let field_tv = children
            .iter()
            .find(|&&child| {
                qctx.get::<NodeKind>(child) == Some(&NodeKind::Field)
                    && qctx
                        .get::<Name>(child)
                        .is_some_and(|n| n.0 == field.field_name)
            })
            .and_then(|&child| {
                qctx.query(LowerTypeAnnotation { entity: child, root })
                    .map(|hir_ty| lower_hir_ty(ctx, &hir_ty))
            })
            .unwrap_or_else(|| ctx.fresh());

        if let Some(pat) = field.pattern {
            gen_pat(ctx, hir, pat, field_tv);
        }
    }
}

