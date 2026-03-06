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

use kestrel_ast_builder::{Callable, TypeAnnotation};
use kestrel_hecs::{Entity, QueryContext, QueryFn};
use kestrel_hir_lower::LowerBody;

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
        // Self type = the parent type entity
        let self_tv = if let Some(parent) = query_ctx.parent_of(entity) {
            ctx.named(parent, vec![])
        } else {
            ctx.fresh()
        };
        param_tvs.push(self_tv);
    }

    // Create TyVars for each declared parameter
    for param in &callable.params {
        let tv = if let Some(ast_ty) = &param.ty {
            // Parameter has a type annotation — resolve AstType → HirTy → TyVar
            let hir_ty = generate::lower_ast_type(query_ctx, entity, ctx.root, ast_ty);
            generate::lower_hir_ty(ctx, &hir_ty)
        } else {
            ctx.fresh()
        };
        param_tvs.push(tv);
    }

    param_tvs
}

/// Create return type TyVar from the TypeAnnotation component.
fn create_return_type(
    ctx: &mut InferCtx<'_>,
    query_ctx: &QueryContext<'_>,
    entity: Entity,
) -> ty::TyVar {
    // If entity has a TypeAnnotation, use it as the return type
    if let Some(type_ann) = query_ctx.get::<TypeAnnotation>(entity) {
        let hir_ty = generate::lower_ast_type(query_ctx, entity, ctx.root, &type_ann.0);
        generate::lower_hir_ty(ctx, &hir_ty)
    } else {
        // No return type annotation — infer from body
        ctx.fresh()
    }
}
