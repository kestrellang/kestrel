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

pub mod constraint;
pub mod ctx;
pub mod error;
pub mod generate;
pub mod resolve;
pub mod result;
pub mod solver;
pub mod ty;
pub mod unify;

use kestrel_ast_builder::Callable;
use kestrel_hecs::{Entity, QueryContext, QueryFn};
use kestrel_hir_lower::{LowerBody, LowerCallableTypes, LowerTypeAnnotation};

use ctx::InferCtx;
use resolve::WorldResolver;
use result::TypedBody;

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
            ctx: query_ctx,
            root: self.root,
            owner: self.entity,
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
        solver::solve(&mut infer_ctx);

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
        let self_tv = if let Some(parent) = query_ctx.parent_of(entity) {
            let parent_kind = query_ctx.get::<kestrel_ast_builder::NodeKind>(parent);
            if parent_kind == Some(&kestrel_ast_builder::NodeKind::Extension) {
                // Resolve extension target to the actual type
                match query_ctx.query(kestrel_name_res::ExtensionTargetEntity {
                    extension: parent,
                    root: ctx.root,
                }) {
                    Some(target) => {
                        let fresh_args = fresh_type_args(ctx, query_ctx, target);
                        ctx.named(target, fresh_args)
                    }
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

    param_tvs
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

/// Create fresh TyVars for each type parameter of an entity.
fn fresh_type_args(
    ctx: &mut InferCtx<'_>,
    query_ctx: &QueryContext<'_>,
    entity: Entity,
) -> Vec<ty::TyVar> {
    query_ctx
        .get::<kestrel_ast_builder::TypeParams>(entity)
        .map(|tp| tp.0.iter().map(|_| ctx.fresh()).collect())
        .unwrap_or_default()
}
