//! kestrel-type-infer: Bidirectional constraint-based type inference.
//!
//! Takes `HirBody` (from HIR lowering) and produces `TypedBody` with
//! resolved types, member resolutions, and promotions. Uses a fixpoint
//! solver over 6 constraint types with Hindley-Milner-style unification.
//!
//! Pipeline position:
//! ```text
//! HIR Lowering (HirBody) → Type Inference (this crate) → TypedBody
//! ```

pub mod compare;
pub mod constraint;
pub mod ctx;
pub mod error;
pub mod generate;
pub mod resolve;
pub mod result;
pub mod solver;
pub mod ty;
pub mod unify;
pub mod where_clauses;

use std::collections::HashMap;

use kestrel_ast_builder::{Callable, NodeKind, TypeParams};
use kestrel_hecs::{Entity, QueryContext, QueryFn};
use kestrel_hir_lower::{
    LowerBody, LowerCallableTypes, LowerExtensionTargetTypeArgs, LowerTypeAnnotation,
};
use kestrel_span::Span;

use ctx::InferCtx;
use resolve::WorldResolver;
use result::TypedBody;

/// Resolve the logical enclosing container for a function-like entity.
/// For a Setter (child of Field/Subscript), skip through the accessor-owner
/// parent so the true container (Struct/Enum/Extension/Protocol/Module) is
/// used for `self` typing, type-param inheritance, and where-clause resolution.
/// For plain Functions/Initializers/Deinits/Fields/Subscripts, returns the
/// direct parent unchanged.
fn accessor_enclosing_container(qctx: &QueryContext<'_>, entity: Entity) -> Option<Entity> {
    let direct = qctx.parent_of(entity)?;
    match qctx.get::<NodeKind>(direct) {
        Some(NodeKind::Field) | Some(NodeKind::Subscript) => qctx.parent_of(direct),
        _ => Some(direct),
    }
}

// ===== InferBody query =====

/// Query: infer types for a function/init/getter body.
///
/// Reads HirBody (from LowerBody query), generates constraints,
/// solves them, and returns a TypedBody with resolved types.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct InferBody {
    pub entity: Entity,
    pub root: Entity,
}

impl QueryFn for InferBody {
    type Output = Option<TypedBody>;

    fn describe(&self) -> String {
        format!("InferBody(entity={:?})", self.entity)
    }

    fn execute(&self, query_ctx: &QueryContext<'_>) -> Option<TypedBody> {
        // Get the HIR body
        let hir = query_ctx.query(LowerBody {
            entity: self.entity,
            root: self.root,
        })?;

        // Create the type resolver
        let resolver = WorldResolver {
            // see TypeResolver trait doc — body_owner is the body being inferred
            ctx: query_ctx,
            root: self.root,
            body_owner: self.entity,
        };

        // Create inference context
        let mut infer_ctx = InferCtx::new(&resolver, query_ctx, self.entity, self.root);

        // Create TyVars for params based on Callable component
        let param_types = create_param_types(&mut infer_ctx, query_ctx, self.entity);

        // Create return type TyVar from TypeAnnotation component
        let return_ty = create_return_type(&mut infer_ctx, query_ctx, self.entity);

        // Generate constraints from HIR
        generate::generate(&mut infer_ctx, &hir, &param_types, return_ty);

        // Solve
        solver::solve(&mut infer_ctx, &hir);

        // Build output
        Some(result::build_result(&infer_ctx))
    }
}

/// Create TyVars for function parameters based on the Callable component.
fn create_param_types(
    ctx: &mut InferCtx<'_>,
    query_ctx: &QueryContext<'_>,
    entity: Entity,
) -> Vec<ty::TyVar> {
    let Some(callable) = query_ctx.get::<Callable>(entity) else {
        return Vec::new();
    };

    let mut param_tvs = Vec::new();

    // If method has a receiver, create self type
    if callable.receiver.is_some() {
        // Self type = the parent type entity with fresh TyVars for type params.
        // For methods in extensions, resolve to the extension's target type.
        // Setters are children of Field/Subscript — skip one more hop to the
        // actual enclosing type (Struct/Enum/Extension/Protocol/Module).
        let self_tv = if let Some(parent) = accessor_enclosing_container(query_ctx, entity) {
            let parent_kind = query_ctx.get::<NodeKind>(parent);
            if parent_kind == Some(&NodeKind::Extension) {
                // Resolve extension target to the actual type
                match query_ctx.query(kestrel_name_res::ExtensionTargetEntity {
                    extension: parent,
                    root: ctx.root,
                }) {
                    Some(target) => {
                        // Get explicit type args from the extension target (e.g., [lang.i64] in extend Box[lang.i64])
                        let ext_type_args = query_ctx.query(LowerExtensionTargetTypeArgs {
                            extension: parent,
                            root: ctx.root,
                        });

                        // Build args: use concrete type args where provided, fresh TyVars elsewhere
                        let type_params: Vec<Entity> = query_ctx
                            .get::<TypeParams>(target)
                            .map(|tp| tp.0.clone())
                            .unwrap_or_default();

                        let args: Vec<ty::TyVar> = if let Some(ref hir_args) = ext_type_args {
                            if !hir_args.is_empty() {
                                // Extension has explicit type args — use them
                                type_params
                                    .iter()
                                    .enumerate()
                                    .map(|(i, &param_entity)| {
                                        if let Some(hir_ty) = hir_args.get(i) {
                                            generate::lower_hir_ty(ctx, hir_ty)
                                        } else {
                                            ctx.param(param_entity)
                                        }
                                    })
                                    .collect()
                            } else {
                                fresh_type_args(ctx, query_ctx, target)
                            }
                        } else {
                            fresh_type_args(ctx, query_ctx, target)
                        };

                        // For protocol extensions, `self` is the abstract Self
                        // of the protocol — not the protocol entity itself. Use
                        // `SelfType(P)` so it round-trips through inference output
                        // as `ResolvedTy::SelfType` → `MirTy::SelfType` and gets
                        // substituted per-concrete-receiver at monomorphization.
                        // Other kinds (Struct/Enum) emit the concrete target.
                        let target_kind = query_ctx.get::<NodeKind>(target).cloned();
                        let self_tv = if matches!(target_kind, Some(NodeKind::Protocol)) {
                            ctx.self_type_ty(target)
                        } else {
                            ctx.named(target, args.clone())
                        };

                        // Emit extension where clause constraints so the solver
                        // knows about bounds like Item: Addable, Item.Output = Item
                        emit_extension_where_clauses(
                            ctx, query_ctx, parent, target, &args, self_tv,
                        );

                        self_tv
                    },
                    None => ctx.fresh(),
                }
            } else {
                let fresh_args = fresh_type_args(ctx, query_ctx, parent);
                ctx.named(parent, fresh_args)
            }
        } else {
            ctx.fresh()
        };
        param_tvs.push(self_tv);
    }

    // Create TyVars for each declared parameter via lowered types
    let lowered = query_ctx.query(LowerCallableTypes {
        entity,
        root: ctx.root,
    });
    if let Some(hir_tys) = &lowered {
        for t in hir_tys {
            let tv = match t {
                Some(hir_ty) => generate::lower_hir_ty(ctx, hir_ty),
                None => ctx.fresh(),
            };
            param_tvs.push(tv);
        }
    }

    // Emit the method's own where clause constraints (e.g., `where I: Iterable, V = Array[E]`).
    // Extension where clauses are handled by emit_extension_where_clauses above;
    // this handles where clauses declared on the method/init itself.
    emit_method_where_clauses(ctx, query_ctx, entity);

    param_tvs
}

/// Emit where clause constraints for the method entity's own type parameters.
/// Handles bounds (`I: Iterable`), associated type equalities (`I.Item = E`),
/// and direct type param equalities (`V = Array[E]`).
fn emit_method_where_clauses(ctx: &mut InferCtx<'_>, query_ctx: &QueryContext<'_>, entity: Entity) {
    let clauses = query_ctx.query(crate::where_clauses::WhereClausesOf {
        entity,
        root: ctx.root,
    });
    if clauses.is_empty() {
        return;
    }

    // Build entity → TyVar mapping for type params already created (by lower_hir_ty).
    // Method type params get Param TyVars; struct type params get Param TyVars from fresh_type_args.
    // We need to find the existing TyVars for each type param entity.
    let type_params: Vec<Entity> = query_ctx
        .get::<kestrel_ast_builder::TypeParams>(entity)
        .map(|tp| tp.0.clone())
        .unwrap_or_default();

    // Also include the parent struct's type params (for where clauses like `V = Array[E]`)
    let parent_type_params: Vec<Entity> = query_ctx
        .parent_of(entity)
        .and_then(|p| query_ctx.get::<kestrel_ast_builder::TypeParams>(p))
        .map(|tp| tp.0.clone())
        .unwrap_or_default();

    let span = Span::synthetic(0);

    for clause in clauses {
        match clause {
            resolve::WhereClause::Bound {
                param, protocol, ..
            } => {
                // Find or create a TyVar for this param
                let tv = ctx.param(param);
                ctx.conforms(tv, protocol, span.clone());
            },
            resolve::WhereClause::TypeEquality {
                param,
                assoc_name,
                rhs,
            } => {
                let subject_tv = ctx.param(param);
                let assoc_result = ctx.fresh();
                ctx.associated(subject_tv, &assoc_name, assoc_result, span.clone());

                // Build subs for type params
                let mut subs: Vec<(Entity, ty::TyVar)> = Vec::new();
                for &tp in type_params.iter().chain(parent_type_params.iter()) {
                    subs.push((tp, ctx.param(tp)));
                }
                let rhs_tv = generate::lower_hir_ty_with_subs(ctx, &rhs, &subs);
                ctx.equal(assoc_result, rhs_tv, span.clone());

                if let Some(assoc_entity) = find_assoc_type_in_bounds(ctx, param, &assoc_name) {
                    ctx.where_clause_assoc_subs.push((assoc_entity, rhs_tv));
                }
            },
            resolve::WhereClause::DirectEquality { param, rhs } => {
                let param_tv = ctx.param(param);
                // Build subs for type params
                let mut subs: Vec<(Entity, ty::TyVar)> = Vec::new();
                for &tp in type_params.iter().chain(parent_type_params.iter()) {
                    subs.push((tp, ctx.param(tp)));
                }
                let rhs_tv = generate::lower_hir_ty_with_subs(ctx, &rhs, &subs);
                // Redirect the param to the RHS type
                ctx.types[param_tv.0 as usize] = ty::TySlot::Redirect(rhs_tv);
                // For TypeAlias entities (associated types like Item), also register
                // in where_clause_assoc_subs so lower_hir_ty_sub can find it
                if query_ctx.get::<kestrel_ast_builder::NodeKind>(param)
                    == Some(&kestrel_ast_builder::NodeKind::TypeAlias)
                {
                    ctx.where_clause_assoc_subs.push((param, rhs_tv));
                }
            },
        }
    }
}

/// Create return type TyVar from the TypeAnnotation component.
fn create_return_type(
    ctx: &mut InferCtx<'_>,
    query_ctx: &QueryContext<'_>,
    entity: Entity,
) -> ty::TyVar {
    query_ctx
        .query(LowerTypeAnnotation {
            entity,
            root: ctx.root,
        })
        .map(|hir_ty| generate::lower_hir_ty(ctx, &hir_ty))
        .unwrap_or_else(|| ctx.fresh())
}

/// Create Param TyVars for the struct's type parameters in method body setup.
/// Params are concrete, enabling protocol-based member resolution for generic code
/// (e.g., V.add() resolves via V: Addable).
fn fresh_type_args(
    ctx: &mut InferCtx<'_>,
    query_ctx: &QueryContext<'_>,
    entity: Entity,
) -> Vec<ty::TyVar> {
    query_ctx
        .get::<kestrel_ast_builder::TypeParams>(entity)
        .map(|tp| tp.0.iter().map(|&param| ctx.param(param)).collect())
        .unwrap_or_default()
}

/// Emit extension where clause constraints for the method body being inferred.
///
/// Extension where clauses (e.g., `extend Iterator where Item: Addable, Item.Output = Item`)
/// need to be emitted as constraints so the solver knows about bounds on associated types
/// and type parameters when inferring method bodies inside the extension.
fn emit_extension_where_clauses(
    ctx: &mut InferCtx<'_>,
    query_ctx: &QueryContext<'_>,
    extension: Entity,
    target: Entity,
    fresh_args: &[ty::TyVar],
    self_tv: ty::TyVar,
) {
    // Build type param entity → TyVar map for the target type's params
    let target_type_params: Vec<Entity> = query_ctx
        .get::<kestrel_ast_builder::TypeParams>(target)
        .map(|tp| tp.0.clone())
        .unwrap_or_default();

    // Cache for associated type TyVars so we reuse the same TyVar
    // if the same associated type appears in multiple constraints
    let mut assoc_type_tvs: HashMap<Entity, ty::TyVar> = HashMap::new();

    let clauses = query_ctx.query(crate::where_clauses::WhereClausesOf {
        entity: extension,
        root: ctx.root,
    });
    for clause in clauses {
        match clause {
            resolve::WhereClause::Bound {
                param, protocol, ..
            } => {
                let span = Span::synthetic(0);
                let subject_tv = get_or_create_subject_tv(
                    ctx,
                    &target_type_params,
                    fresh_args,
                    &mut assoc_type_tvs,
                    param,
                    self_tv,
                    query_ctx,
                );
                ctx.conforms(subject_tv, protocol, span.clone());

                // Propagate where clauses from the protocol's associated types.
                // E.g., `type Iter: Iterator where Iter.Item = Item` on Iterable
                // needs to emit `Associated(T.Iter, "Item", fresh) + Equal(fresh, T.Item)`.
                emit_protocol_assoc_type_where_clauses(
                    ctx,
                    query_ctx,
                    protocol,
                    subject_tv,
                    &target_type_params,
                    fresh_args,
                    &mut assoc_type_tvs,
                    self_tv,
                    &span,
                );
            },
            resolve::WhereClause::TypeEquality {
                param,
                assoc_name,
                rhs,
            } => {
                let span = Span::synthetic(0);
                let subject_tv = get_or_create_subject_tv(
                    ctx,
                    &target_type_params,
                    fresh_args,
                    &mut assoc_type_tvs,
                    param,
                    self_tv,
                    query_ctx,
                );
                let assoc_result = ctx.fresh();
                ctx.associated(subject_tv, &assoc_name, assoc_result, span.clone());

                // Build subs so RHS references to type params and associated types
                // resolve to the same TyVars used in constraints (not raw Named entities)
                let mut rhs_subs: Vec<(Entity, ty::TyVar)> = target_type_params
                    .iter()
                    .zip(fresh_args.iter())
                    .map(|(&e, &tv)| (e, tv))
                    .collect();
                for (&e, &tv) in &assoc_type_tvs {
                    rhs_subs.push((e, tv));
                }
                let rhs_tv = generate::lower_hir_ty_with_subs(ctx, &rhs, &rhs_subs);
                ctx.equal(assoc_result, rhs_tv, span);

                // Register the associated type entity → rhs_tv mapping so the solver
                // can substitute it in protocol member signatures (e.g., Output → Item).
                // Search param's protocol bounds for a child TypeAlias named assoc_name.
                if let Some(assoc_entity) = find_assoc_type_in_bounds(ctx, param, &assoc_name) {
                    ctx.where_clause_assoc_subs.push((assoc_entity, rhs_tv));
                }
            },
            resolve::WhereClause::DirectEquality { param, rhs } => {
                // Direct type param equality: V = Array[E]
                // Overwrite the param's TyVar slot with the concrete RHS type.
                if let Some(idx) = target_type_params.iter().position(|&p| p == param) {
                    let param_tv = fresh_args[idx];
                    // Build subs so RHS type param references resolve correctly
                    let mut rhs_subs: Vec<(Entity, ty::TyVar)> = target_type_params
                        .iter()
                        .zip(fresh_args.iter())
                        .map(|(&e, &tv)| (e, tv))
                        .collect();
                    for (&e, &tv) in &assoc_type_tvs {
                        rhs_subs.push((e, tv));
                    }
                    let rhs_tv = generate::lower_hir_ty_with_subs(ctx, &rhs, &rhs_subs);
                    // Overwrite the Param slot → the param IS the RHS type in this scope
                    ctx.types[param_tv.0 as usize] = ty::TySlot::Redirect(rhs_tv);
                }
            },
        }
    }
}

/// Propagate where clauses from a protocol's associated type declarations.
///
/// When `T: Iterable` and `Iterable` has `type Iter: Iterator where Iter.Item = Item`,
/// this emits constraints connecting `T.Iter.Item` to `T.Item` through the TypeAlias
/// where clause. Without this, `T.Iter.Item` and `T.Item` would be unrelated TyVars.
fn emit_protocol_assoc_type_where_clauses(
    ctx: &mut InferCtx<'_>,
    query_ctx: &QueryContext<'_>,
    protocol: Entity,
    subject_tv: ty::TyVar,
    target_type_params: &[Entity],
    fresh_args: &[ty::TyVar],
    assoc_type_tvs: &mut HashMap<Entity, ty::TyVar>,
    _self_tv: ty::TyVar,
    span: &Span,
) {
    // Walk the protocol's children looking for TypeAlias entities with where clauses
    for &child in query_ctx.children_of(protocol) {
        if query_ctx.get::<kestrel_ast_builder::NodeKind>(child)
            != Some(&kestrel_ast_builder::NodeKind::TypeAlias)
        {
            continue;
        }

        // Read where clauses attached to `child`. Resolution uses child's own
        // scope (scope walks child → protocol), so siblings like `Item` are
        // reachable without a separate context parameter.
        let _ = protocol; // retained by outer scope; not used for resolution
        let clauses = query_ctx.query(crate::where_clauses::WhereClausesOf {
            entity: child,
            root: ctx.root,
        });
        if clauses.is_empty() {
            continue;
        }

        // Get the TyVar for this associated type (e.g., T.Iter)
        let assoc_name = query_ctx
            .get::<kestrel_ast_builder::Name>(child)
            .map(|n| n.0.clone())
            .unwrap_or_default();
        let alias_tv = *assoc_type_tvs.entry(child).or_insert_with(|| {
            let tv = ctx.fresh();
            ctx.associated(subject_tv, &assoc_name, tv, span.clone());
            ctx.where_clause_assoc_subs.push((child, tv));
            tv
        });

        // Emit the TypeAlias's own where clauses
        for clause in clauses {
            match clause {
                resolve::WhereClause::Bound {
                    protocol: bound_proto,
                    ..
                } => {
                    ctx.conforms(alias_tv, bound_proto, span.clone());
                },
                resolve::WhereClause::TypeEquality {
                    assoc_name: inner_assoc,
                    rhs,
                    ..
                } => {
                    let fresh = ctx.fresh();
                    ctx.associated(alias_tv, &inner_assoc, fresh, span.clone());

                    // Build subs so RHS references resolve correctly
                    let mut rhs_subs: Vec<(Entity, ty::TyVar)> = target_type_params
                        .iter()
                        .zip(fresh_args.iter())
                        .map(|(&e, &tv)| (e, tv))
                        .collect();
                    for (&e, &tv) in assoc_type_tvs.iter() {
                        rhs_subs.push((e, tv));
                    }
                    let rhs_tv = generate::lower_hir_ty_with_subs(ctx, &rhs, &rhs_subs);
                    ctx.equal(fresh, rhs_tv, span.clone());

                    // Register so solve_associated can reuse
                    if let Some(inner_entity) = find_assoc_type_in_bounds(ctx, child, &inner_assoc)
                    {
                        ctx.where_clause_assoc_subs.push((inner_entity, rhs_tv));
                    }
                },
                resolve::WhereClause::DirectEquality { .. } => {},
            }
        }
    }
}

/// Get or create a TyVar for a where clause subject entity.
///
/// If the entity is a type parameter of the target type, return its fresh TyVar.
/// If it's an associated type (TypeAlias), create an `associated` constraint
/// on self_tv and return the result TyVar (cached for reuse).
fn get_or_create_subject_tv(
    ctx: &mut InferCtx<'_>,
    target_type_params: &[Entity],
    fresh_args: &[ty::TyVar],
    assoc_type_tvs: &mut HashMap<Entity, ty::TyVar>,
    param: Entity,
    self_tv: ty::TyVar,
    query_ctx: &QueryContext<'_>,
) -> ty::TyVar {
    // Check if param is a type parameter of the target type
    if let Some(idx) = target_type_params.iter().position(|&p| p == param) {
        if idx < fresh_args.len() {
            return fresh_args[idx];
        }
    }

    // Check if param is an associated type (TypeAlias) — create via associated constraint
    if query_ctx.get::<NodeKind>(param) == Some(&NodeKind::TypeAlias) {
        if let Some(&cached) = assoc_type_tvs.get(&param) {
            return cached;
        }
        // Get the name of the associated type
        if let Some(name) = query_ctx.get::<kestrel_ast_builder::Name>(param) {
            let result_tv = ctx.fresh();
            ctx.associated(self_tv, &name.0, result_tv, Span::synthetic(0));
            assoc_type_tvs.insert(param, result_tv);
            return result_tv;
        }
    }

    // Fallback: create a fresh TyVar
    ctx.fresh()
}

/// Find an associated type entity by searching protocol bounds of a TypeAlias.
/// E.g., for param=Item (which conforms to Addable), find Addable's "Output" child.
/// Uses the resolver to find protocol bounds, then searches their children.
fn find_assoc_type_in_bounds(
    ctx: &InferCtx<'_>,
    param: Entity,
    assoc_name: &str,
) -> Option<Entity> {
    // Resolve the associated type through the resolver's associated type mechanism.
    // Build a TyKind::Param for the param entity to query the resolver.
    let param_kind = ty::TyKind::Param { entity: param };
    let resolved = ctx
        .resolver
        .resolve_associated_type(&param_kind, assoc_name)?;
    // Extract the entity from whichever variant the resolver returned.
    match &resolved.resolved {
        kestrel_hir::ty::HirTy::Struct { entity, .. }
        | kestrel_hir::ty::HirTy::Enum { entity, .. }
        | kestrel_hir::ty::HirTy::Protocol { entity, .. }
        | kestrel_hir::ty::HirTy::AliasUse { entity, .. } => Some(*entity),
        kestrel_hir::ty::HirTy::AssocProjection { assoc, .. } => Some(*assoc),
        _ => None,
    }
}
