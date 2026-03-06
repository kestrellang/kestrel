//! Constraint generation: walk HirBody once, emitting constraints for every node.
//!
//! This is a one-pass walk that creates TyVars for every expression,
//! statement binding, and local variable, then emits constraints
//! capturing the type relationships between them.

use kestrel_ast_builder::{AstType, Callable, Name, NodeKind, TypeAnnotation, TypeParams};
use kestrel_hecs::{Entity, QueryContext};
use kestrel_hir::body::*;
use kestrel_hir::ty::HirTy;
use kestrel_name_res::{ResolveTypePath, TypeResolution};
use kestrel_span2::Span;

use crate::constraint::CallArg;
use crate::ctx::InferCtx;
use crate::error::InferError;
use kestrel_hir::Builtin;
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
            let callee_tv = gen_expr(ctx, hir, *callee);
            let arg_tvs = gen_call_args(ctx, hir, args);
            let result_tv = ctx.fresh();

            // Build expected function type: (arg_tys) -> result_tv
            let param_tvs: Vec<TyVar> = arg_tvs.iter().map(|a| a.ty).collect();
            let fn_tv = ctx.function(param_tvs, result_tv);

            // Callee type must match the function type
            ctx.equal(callee_tv, fn_tv, span.clone());
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
            // Condition must be Bool
            let cond_tv = gen_expr(ctx, hir, *condition);
            if let Some(bool_entity) = ctx.resolver.builtin(Builtin::Bool) {
                let bool_tv = ctx.named(bool_entity, vec![]);
                ctx.equal(cond_tv, bool_tv, span.clone());
            }

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
                    let guard_tv = gen_expr(ctx, hir, guard);
                    if let Some(bool_entity) = ctx.resolver.builtin(Builtin::Bool) {
                        let bool_tv = ctx.named(bool_entity, vec![]);
                        ctx.equal(guard_tv, bool_tv, span.clone());
                    }
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
                    let rhs_tv = crate::solver::kind_to_tyvar(ctx, &rhs);
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
            if let Some(callable) = qctx.get::<Callable>(entity) {
                let param_tvs: Vec<TyVar> = callable
                    .params
                    .iter()
                    .map(|p| {
                        if let Some(ast_ty) = &p.ty {
                            let hir_ty = lower_ast_type(qctx, entity, root, ast_ty);
                            lower_hir_ty(ctx, &hir_ty)
                        } else {
                            ctx.fresh()
                        }
                    })
                    .collect();

                let ret_tv = if let Some(type_ann) = qctx.get::<TypeAnnotation>(entity) {
                    let hir_ty = lower_ast_type(qctx, entity, root, &type_ann.0);
                    lower_hir_ty(ctx, &hir_ty)
                } else {
                    ctx.fresh()
                };

                ctx.function(param_tvs, ret_tv)
            } else {
                // Callable without params (shouldn't happen for functions)
                ctx.named(entity, fresh_type_args)
            }
        }

        // Enum case: build function type (payload → parent enum) or unit
        Some(NodeKind::EnumCase) => {
            let parent_enum = qctx.parent_of(entity);

            if let Some(callable) = qctx.get::<Callable>(entity) {
                // Case with payload → function from payload to enum type
                let param_tvs: Vec<TyVar> = callable
                    .params
                    .iter()
                    .map(|p| {
                        if let Some(ast_ty) = &p.ty {
                            let hir_ty = lower_ast_type(qctx, entity, root, ast_ty);
                            lower_hir_ty(ctx, &hir_ty)
                        } else {
                            ctx.fresh()
                        }
                    })
                    .collect();

                // Return type is the parent enum
                let ret_tv = if let Some(pe) = parent_enum {
                    // Get parent's type params and create corresponding fresh TyVars
                    let parent_tps: Vec<Entity> = qctx
                        .get::<TypeParams>(pe)
                        .map(|tp| tp.0.clone())
                        .unwrap_or_default();
                    let parent_args: Vec<TyVar> =
                        parent_tps.iter().map(|_| ctx.fresh()).collect();
                    ctx.named(pe, parent_args)
                } else {
                    ctx.fresh()
                };

                ctx.function(param_tvs, ret_tv)
            } else {
                // Unit case (no payload) → the parent enum type
                if let Some(pe) = parent_enum {
                    let parent_tps: Vec<Entity> = qctx
                        .get::<TypeParams>(pe)
                        .map(|tp| tp.0.clone())
                        .unwrap_or_default();
                    let parent_args: Vec<TyVar> =
                        parent_tps.iter().map(|_| ctx.fresh()).collect();
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
        Some(NodeKind::Field) => {
            if let Some(type_ann) = qctx.get::<TypeAnnotation>(entity) {
                let hir_ty = lower_ast_type(qctx, entity, root, &type_ann.0);
                lower_hir_ty(ctx, &hir_ty)
            } else {
                ctx.fresh()
            }
        }

        // Default: Named type wrapping the entity
        _ => ctx.named(entity, fresh_type_args),
    }
}

/// Convert an HirTy (already resolved during HIR lowering) to a TyVar.
pub fn lower_hir_ty(ctx: &mut InferCtx<'_>, ty: &HirTy) -> TyVar {
    match ty {
        HirTy::Named { entity, args, .. } => {
            let arg_tvs: Vec<TyVar> = args.iter().map(|a| lower_hir_ty(ctx, a)).collect();
            ctx.named(*entity, arg_tvs)
        }
        HirTy::Tuple(types, _) => {
            let elem_tvs: Vec<TyVar> = types.iter().map(|t| lower_hir_ty(ctx, t)).collect();
            ctx.tuple(elem_tvs)
        }
        HirTy::Function { params, ret, .. } => {
            let param_tvs: Vec<TyVar> = params.iter().map(|p| lower_hir_ty(ctx, p)).collect();
            let ret_tv = lower_hir_ty(ctx, ret);
            ctx.function(param_tvs, ret_tv)
        }
        HirTy::Param(entity, _) => ctx.param(*entity),
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
    let payload_types: Vec<TyVar> = if let Some(callable) = qctx.get::<Callable>(entity) {
        callable
            .params
            .iter()
            .map(|p| {
                if let Some(ast_ty) = &p.ty {
                    let hir_ty = lower_ast_type(qctx, entity, root, ast_ty);
                    lower_hir_ty(ctx, &hir_ty)
                } else {
                    ctx.fresh()
                }
            })
            .collect()
    } else {
        // No Callable → unit case, args should be empty
        vec![]
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
                qctx.get::<TypeAnnotation>(child).map(|ann| {
                    let hir_ty = lower_ast_type(qctx, entity, root, &ann.0);
                    lower_hir_ty(ctx, &hir_ty)
                })
            })
            .unwrap_or_else(|| ctx.fresh());

        if let Some(pat) = field.pattern {
            gen_pat(ctx, hir, pat, field_tv);
        }
    }
}

// ===== AstType → HirTy conversion =====

/// Convert an AstType to HirTy using name resolution.
/// Mirrors `kestrel-hir-lower/src/ty.rs`'s `lower_type()` logic
/// but as a standalone function (no LowerCtx needed).
pub fn lower_ast_type(
    ctx: &QueryContext<'_>,
    owner: Entity,
    root: Entity,
    ty: &AstType,
) -> HirTy {
    match ty {
        AstType::Named { segments, span } => {
            let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
            let result = ctx.query(ResolveTypePath {
                segments: seg_names,
                context: owner,
                root,
            });

            match result {
                TypeResolution::Found(entity) => {
                    // Type parameter → HirTy::Param
                    if ctx.get::<NodeKind>(entity) == Some(&NodeKind::TypeParameter) {
                        return HirTy::Param(entity, span.clone());
                    }

                    // Lower type arguments from all segments
                    let args: Vec<HirTy> = segments
                        .iter()
                        .flat_map(|s| s.type_args.iter())
                        .map(|a| lower_ast_type(ctx, owner, root, a))
                        .collect();

                    HirTy::Named {
                        entity,
                        args,
                        span: span.clone(),
                    }
                }
                TypeResolution::SelfType => {
                    // Walk up from owner to find enclosing type
                    if let Some(self_entity) = find_self_type(ctx, owner) {
                        HirTy::Named {
                            entity: self_entity,
                            args: Vec::new(),
                            span: span.clone(),
                        }
                    } else {
                        HirTy::Error(span.clone())
                    }
                }
                TypeResolution::NotFound(_) | TypeResolution::NotAType(_) => {
                    HirTy::Error(span.clone())
                }
            }
        }

        AstType::Tuple(types, span) => {
            let lowered: Vec<HirTy> = types
                .iter()
                .map(|t| lower_ast_type(ctx, owner, root, t))
                .collect();
            HirTy::Tuple(lowered, span.clone())
        }

        AstType::Function {
            params,
            return_type,
            span,
        } => {
            let lowered_params: Vec<HirTy> = params
                .iter()
                .map(|t| lower_ast_type(ctx, owner, root, t))
                .collect();
            let lowered_ret = Box::new(lower_ast_type(ctx, owner, root, return_type));
            HirTy::Function {
                params: lowered_params,
                ret: lowered_ret,
                span: span.clone(),
            }
        }

        // Sugar types: resolve stdlib entity + Named
        AstType::Array(elem, span) => lower_sugar_type(ctx, owner, root, "Array", &[elem], span),
        AstType::Optional(inner, span) => {
            lower_sugar_type(ctx, owner, root, "Optional", &[inner], span)
        }
        AstType::Dictionary(key, val, span) => {
            lower_sugar_type(ctx, owner, root, "Dictionary", &[key, val], span)
        }
        AstType::Result { ok, err, span } => {
            lower_sugar_type(ctx, owner, root, "Result", &[ok, err], span)
        }
        AstType::Unit(span) => HirTy::Tuple(Vec::new(), span.clone()),
        AstType::Never(span) => {
            if let Some(entity) = resolve_std_type(ctx, owner, root, "Never") {
                HirTy::Named {
                    entity,
                    args: Vec::new(),
                    span: span.clone(),
                }
            } else {
                HirTy::Error(span.clone())
            }
        }
        AstType::Inferred(span) => HirTy::Infer(span.clone()),
    }
}

/// Lower a sugar type by resolving the stdlib entity.
fn lower_sugar_type(
    ctx: &QueryContext<'_>,
    owner: Entity,
    root: Entity,
    name: &str,
    type_args: &[&Box<AstType>],
    span: &Span,
) -> HirTy {
    let lowered_args: Vec<HirTy> = type_args
        .iter()
        .map(|t| lower_ast_type(ctx, owner, root, t))
        .collect();

    if let Some(entity) = resolve_std_type(ctx, owner, root, name) {
        HirTy::Named {
            entity,
            args: lowered_args,
            span: span.clone(),
        }
    } else {
        HirTy::Error(span.clone())
    }
}

/// Resolve a well-known stdlib type name to an entity.
fn resolve_std_type(
    ctx: &QueryContext<'_>,
    owner: Entity,
    root: Entity,
    name: &str,
) -> Option<Entity> {
    match ctx.query(ResolveTypePath {
        segments: vec![name.to_string()],
        context: owner,
        root,
    }) {
        TypeResolution::Found(entity) => Some(entity),
        _ => None,
    }
}

/// Walk up from entity to find the enclosing type (Struct/Enum/Protocol).
fn find_self_type(ctx: &QueryContext<'_>, entity: Entity) -> Option<Entity> {
    let mut current = Some(entity);
    while let Some(e) = current {
        match ctx.get::<NodeKind>(e) {
            Some(NodeKind::Struct | NodeKind::Enum | NodeKind::Protocol | NodeKind::Extension) => {
                return Some(e);
            }
            _ => current = ctx.parent_of(e),
        }
    }
    None
}
