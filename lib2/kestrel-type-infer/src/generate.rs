//! Constraint generation: walk HirBody once, emitting constraints for every node.
//!
//! This is a one-pass walk that creates TyVars for every expression,
//! statement binding, and local variable, then emits constraints
//! capturing the type relationships between them.

use kestrel_ast_builder::{AstParam, Callable, Name, NodeKind, TypeParams};
use kestrel_hecs::Entity;
use kestrel_hir::body::*;
use kestrel_hir::ty::HirTy;
use kestrel_hir_lower::{LowerCallableTypes, LowerTypeAnnotation};
use kestrel_span2::Span;

use crate::constraint::CallArg;
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

        HirExpr::Def(entity, _) => {
            // Read the entity's type from the world via the resolver.
            // For generic entities, this will instantiate fresh TyVars.
            instantiate_entity(ctx, *entity)
        }

        // === Calls ===
        HirExpr::Call { callee, args, span } => {
            // Struct construction: Def(struct) used as callee
            if let HirExpr::Def(entity, _) = &hir.exprs[*callee] {
                if ctx.query_ctx.get::<NodeKind>(*entity) == Some(&NodeKind::Struct) {
                    let arg_tvs = gen_call_args(ctx, hir, args);
                    return gen_struct_init(ctx, *entity, &arg_tvs, id, span);
                }
            }

            // Emit Call constraint — solver dispatches based on callee type:
            // Function → unify params/return, Named → subscript resolution
            let callee_tv = gen_expr(ctx, hir, *callee);
            let arg_tvs = gen_call_args(ctx, hir, args);
            let result_tv = ctx.fresh();

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

            // Member constraint: receiver.method(args) -> result
            ctx.member(recv_tv, method, arg_tvs, result_tv, id, span.clone());
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
            ctx.member(recv_tv, method, arg_tvs, result_tv, id, span.clone());
            result_tv
        }

        // === Member access ===
        HirExpr::Field {
            base, name, span, ..
        } => {
            let base_tv = gen_expr(ctx, hir, *base);
            let result_tv = ctx.fresh();

            // Member constraint with no args (field/property access)
            ctx.member(base_tv, name, vec![], result_tv, id, span.clone());
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
                // Both branches must agree
                ctx.equal(then_tv, result_tv, span.clone());
                ctx.equal(else_tv, result_tv, span.clone());
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

        HirPat::Tuple { elements, span, .. } => {
            // Scrutinee must be a tuple with matching arity
            let elem_tvs: Vec<TyVar> = elements.iter().map(|_| ctx.fresh()).collect();
            let tuple_tv = ctx.tuple(elem_tvs.clone());
            ctx.equal(scrutinee_tv, tuple_tv, span.clone());

            for (&elem_pat, elem_tv) in elements.iter().zip(elem_tvs) {
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

    // Find initializer children
    let children = qctx.children_of(struct_entity).to_vec();
    let inits: Vec<Entity> = children
        .iter()
        .filter(|&&c| qctx.get::<NodeKind>(c) == Some(&NodeKind::Initializer))
        .copied()
        .collect();

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
        }
        // else: multiple matches — skip arg constraints, solver will disambiguate
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

/// Check if call arg labels match an init's param labels.
fn labels_match(params: &[AstParam], arg_labels: &[Option<&str>]) -> bool {
    if params.len() != arg_labels.len() {
        return false;
    }
    params
        .iter()
        .zip(arg_labels.iter())
        .all(|(param, arg_label)| param.label.as_deref() == *arg_label)
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
    } else {
        ctx.tuple(vec![]) // void block = unit
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
fn instantiate_entity(ctx: &mut InferCtx<'_>, entity: Entity) -> TyVar {
    let qctx = ctx.query_ctx;
    let root = ctx.root;

    // Read type params and create fresh TyVars for them
    let type_param_entities: Vec<Entity> = qctx
        .get::<TypeParams>(entity)
        .map(|tp| tp.0.clone())
        .unwrap_or_default();
    let fresh_type_args: Vec<TyVar> = type_param_entities.iter().map(|_| ctx.fresh()).collect();

    // Build substitution map: type param entity → fresh TyVar.
    // Used to instantiate generic param/return types with fresh vars.
    let subs: Vec<(Entity, TyVar)> = type_param_entities
        .iter()
        .zip(fresh_type_args.iter())
        .map(|(&e, &tv)| (e, tv))
        .collect();

    // Emit where clause constraints for the type params
    for clause in ctx.resolver.where_clauses(entity) {
        match clause {
            crate::resolve::WhereClause::Bound { param, protocol } => {
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
        }
    }

    // Determine entity kind and build appropriate type
    let kind = qctx.get::<NodeKind>(entity);
    match kind {
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

        // Default: Named type wrapping the entity
        _ => ctx.named(entity, fresh_type_args),
    }
}

/// Convert an HirTy (already resolved during HIR lowering) to a TyVar.
pub fn lower_hir_ty(ctx: &mut InferCtx<'_>, ty: &HirTy) -> TyVar {
    lower_hir_ty_with_subs(ctx, ty, &[])
}

/// Convert HirTy to TyVar, substituting type params found in `subs`.
/// Used when instantiating generic entities: type params become fresh TyVars.
fn lower_hir_ty_with_subs(
    ctx: &mut InferCtx<'_>,
    ty: &HirTy,
    subs: &[(Entity, TyVar)],
) -> TyVar {
    match ty {
        HirTy::Named { entity, args, .. } => {
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
fn expr_span(hir: &HirBody, id: HirExprId) -> Span {
    match &hir.exprs[id] {
        HirExpr::Literal { span, .. }
        | HirExpr::Tuple { span, .. }
        | HirExpr::Array { span, .. }
        | HirExpr::Dict { span, .. }
        | HirExpr::Closure { span, .. }
        | HirExpr::Local(_, span)
        | HirExpr::Def(_, span)
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
        | HirExpr::Error { span } => span.clone(),
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

    // Look up parent enum from the case entity
    if let Some(parent_enum) = qctx.parent_of(entity) {
        // Create the enum type with fresh type args
        let parent_tps: Vec<Entity> = qctx
            .get::<TypeParams>(parent_enum)
            .map(|tp| tp.0.clone())
            .unwrap_or_default();
        let parent_args: Vec<TyVar> = parent_tps.iter().map(|_| ctx.fresh()).collect();
        let enum_tv = ctx.named(parent_enum, parent_args);

        // Scrutinee must be this enum type
        ctx.equal(scrutinee_tv, enum_tv, span.clone());
    }

    // Get case's payload types from its Callable component
    let payload_types: Vec<TyVar> =
        match qctx.query(LowerCallableTypes { entity, root }) {
            Some(types) => types
                .iter()
                .map(|t| match t {
                    Some(hir_ty) => lower_hir_ty(ctx, hir_ty),
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
/// Deferred: resolution depends on the scrutinee type being concrete.
/// For now, recurse into sub-patterns with fresh TyVars.
fn gen_implicit_variant_pat(
    ctx: &mut InferCtx<'_>,
    hir: &HirBody,
    _name: &str,
    args: &[HirPatArg],
    _scrutinee_tv: TyVar,
    _span: &Span,
) {
    // Implicit variant resolution requires the scrutinee type to be known.
    // Full implementation would check if scrutinee is concrete, then search
    // enum children for a matching case name. For now, just recurse.
    for arg in args {
        let arg_tv = ctx.fresh();
        gen_pat(ctx, hir, arg.pattern, arg_tv);
    }
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

