//! Type lowering: AstType → HirTy.
//!
//! Resolves type paths to entities and expands sugar types
//! (Array, Optional, Dictionary, Result) into Named types.
//!
//! The standalone `lower_ast_type` function is the shared implementation
//! used both during body lowering (via LowerCtx) and by type inference
//! for declaration-level types (Callable params, TypeAnnotation, etc.).

use kestrel_ast::AstType;
use kestrel_ast_builder::{Callable, NodeKind, TypeAnnotation};
use kestrel_hecs::{Entity, QueryContext, QueryFn};
use kestrel_hir::ty::HirTy;
use kestrel_name_res::{ResolveTypePath, TypeResolution};
use kestrel_span2::Span;

use crate::ctx::LowerCtx;

// ===== Standalone type lowering =====

/// Lower an AstType to HirTy using name resolution.
///
/// This is the shared core used by both body lowering (LowerCtx::lower_type)
/// and declaration-level type queries (LowerTypeAnnotation, LowerCallableTypes).
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

                    // Type alias with concrete definition (e.g., `type Fd = Int32`):
                    // resolve to the aliased type so Fd and Int32 are the same HirTy.
                    // Only for simple aliases (no user-provided type args) with a
                    // concrete TypeAnnotation (not abstract associated types).
                    if ctx.get::<NodeKind>(entity) == Some(&NodeKind::TypeAlias) {
                        let has_user_args = segments.iter().any(|s| !s.type_args.is_empty());
                        if !has_user_args {
                            if let Some(ann) = ctx.get::<TypeAnnotation>(entity) {
                                return lower_ast_type(ctx, owner, root, &ann.0);
                            }
                        }
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
                    if let Some(self_entity) = find_self_type(ctx, owner, root) {
                        HirTy::Named {
                            entity: self_entity,
                            args: Vec::new(),
                            span: span.clone(),
                        }
                    } else {
                        kestrel_debug::ktrace!("hir-lower", "Self type not found in scope");
                        HirTy::Error(span.clone())
                    }
                }
                TypeResolution::NotFound(ref seg) => {
                    kestrel_debug::ktrace!("hir-lower", "type not found: {:?} (failed at {:?})",
                        segments.iter().map(|s| &s.name).collect::<Vec<_>>(), seg);
                    HirTy::Error(span.clone())
                }
                TypeResolution::NotAType(_) => {
                    kestrel_debug::ktrace!("hir-lower", "type not a type: {:?}",
                        segments.iter().map(|s| &s.name).collect::<Vec<_>>());
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

        // Sugar types → resolve standard library entity + Named
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
        AstType::Never(span) => HirTy::Never(span.clone()),
        AstType::Inferred(span) => HirTy::Infer(span.clone()),
    }
}

/// Lower a sugar type (Array, Optional, etc.) by resolving the std type entity.
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
        kestrel_debug::ktrace!("hir-lower", "sugar type not found: {}", name);
        HirTy::Error(span.clone())
    }
}

/// Resolve a well-known standard library type name (e.g. "Array", "Optional").
fn resolve_std_type(
    ctx: &QueryContext<'_>,
    owner: Entity,
    root: Entity,
    name: &str,
) -> Option<Entity> {
    let result = ctx.query(ResolveTypePath {
        segments: vec![name.to_string()],
        context: owner,
        root,
    });
    match result {
        TypeResolution::Found(entity) => Some(entity),
        _ => None,
    }
}

/// Find the enclosing type entity for Self resolution.
/// Walks up from owner to find the nearest Struct/Enum/Protocol.
/// For extensions, resolves to the extension's target type (not the extension itself).
fn find_self_type(ctx: &QueryContext<'_>, owner: Entity, root: Entity) -> Option<Entity> {
    let mut current = Some(owner);
    while let Some(entity) = current {
        match ctx.get::<NodeKind>(entity) {
            Some(NodeKind::Struct | NodeKind::Enum | NodeKind::Protocol) => {
                return Some(entity);
            }
            Some(NodeKind::Extension) => {
                // Resolve to the extension's target type, not the extension itself
                return ctx.query(kestrel_name_res::ExtensionTargetEntity {
                    extension: entity,
                    root,
                });
            }
            _ => {
                current = ctx.parent_of(entity);
            }
        }
    }
    None
}

// ===== LowerCtx delegation =====

impl LowerCtx<'_> {
    /// Lower an AST type to an HIR type. Delegates to the standalone function.
    pub fn lower_type(&mut self, ty: &AstType) -> HirTy {
        lower_ast_type(self.ctx, self.owner, self.root, ty)
    }
}

// ===== Queries =====

/// Query: lower a declaration entity's TypeAnnotation to HirTy.
///
/// Reads the `TypeAnnotation` component and resolves the AstType
/// to an HirTy using name resolution in the entity's scope.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct LowerTypeAnnotation {
    pub entity: Entity,
    pub root: Entity,
}

impl QueryFn for LowerTypeAnnotation {
    type Output = Option<HirTy>;

    fn describe(&self) -> String {
        format!("LowerTypeAnnotation(entity={:?})", self.entity)
    }

    fn execute(&self, ctx: &QueryContext<'_>) -> Option<HirTy> {
        let type_ann = ctx.get::<TypeAnnotation>(self.entity)?;
        Some(lower_ast_type(ctx, self.entity, self.root, &type_ann.0))
    }
}

/// Query: lower a declaration entity's Callable param types to HirTy.
///
/// Reads the `Callable` component and lowers each param's type annotation.
/// Returns a Vec indexed to match `Callable.params` — None for unannotated params.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct LowerCallableTypes {
    pub entity: Entity,
    pub root: Entity,
}

impl QueryFn for LowerCallableTypes {
    type Output = Option<Vec<Option<HirTy>>>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Option<Vec<Option<HirTy>>> {
        let callable = ctx.get::<Callable>(self.entity)?;
        Some(
            callable
                .params
                .iter()
                .map(|p| {
                    p.ty.as_ref()
                        .map(|ast_ty| lower_ast_type(ctx, self.entity, self.root, ast_ty))
                })
                .collect(),
        )
    }
}
