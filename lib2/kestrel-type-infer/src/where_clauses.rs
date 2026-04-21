//! `WhereClausesOf` query — resolves an entity's raw AST where clauses into
//! structured, typed constraints with entities resolved and HIR-lowered RHS.
//!
//! Contract: names in the where clause resolve in the given entity's own
//! scope (via scope walking). No separate `context` parameter — the entity
//! IS the context. The query is memoized per `(entity, root)`.
//!
//! Implemented as free functions (not methods on a stateful resolver) so
//! there is no ambient `self.owner` to accidentally leak into name lookup.

use kestrel_ast_builder::{AstType, NodeKind, WhereClause as AstWhereClause, WhereConstraint};
use kestrel_hecs::{Entity, QueryContext, QueryFn};
use kestrel_hir::ty::HirTy;
use kestrel_name_res::{ResolveTypePath, TypeResolution};

use crate::resolve::WhereClause;

/// Query: resolved where clauses attached to `entity`, with all names looked
/// up in `entity`'s own scope.
///
/// Returns an empty vec if the entity has no where clauses or none resolve.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct WhereClausesOf {
    pub entity: Entity,
    pub root: Entity,
}

impl QueryFn for WhereClausesOf {
    type Output = Vec<WhereClause>;

    fn describe(&self) -> String {
        format!("WhereClausesOf({:?})", self.entity)
    }

    fn execute(&self, ctx: &QueryContext<'_>) -> Vec<WhereClause> {
        resolve_where_clauses(ctx, self.entity, self.root)
    }
}

/// Free-function implementation. Takes `entity` (which is also the resolution
/// context). Separated from the query impl so it can be called directly by
/// other queries without going through the memoization layer when that
/// wouldn't help.
pub fn resolve_where_clauses(
    ctx: &QueryContext<'_>,
    entity: Entity,
    root: Entity,
) -> Vec<WhereClause> {
    let Some(ast_wc) = ctx.get::<AstWhereClause>(entity) else {
        return Vec::new();
    };
    let mut result = Vec::new();
    for constraint in &ast_wc.0 {
        match constraint {
            WhereConstraint::Bound {
                subject, protocols, ..
            } => {
                let Some(param) = resolve_type_entity(ctx, subject, entity, root) else {
                    continue;
                };
                for protocol_ty in protocols {
                    if let Some(protocol) = resolve_type_entity(ctx, protocol_ty, entity, root) {
                        let protocol_type_args =
                            extract_protocol_type_args(ctx, entity, root, protocol_ty);
                        result.push(WhereClause::Bound {
                            param,
                            protocol,
                            protocol_type_args,
                        });
                    }
                }
            },
            WhereConstraint::Equality { lhs, rhs, .. } => {
                let rhs_hir = kestrel_hir_lower::lower_ast_type(ctx, entity, root, rhs);
                if let Some((param, assoc_name)) =
                    extract_associated_type_path(ctx, lhs, entity, root)
                {
                    result.push(WhereClause::TypeEquality {
                        param,
                        assoc_name,
                        rhs: rhs_hir,
                    });
                } else if let Some(param) = resolve_type_param_or_assoc(ctx, lhs, entity, root) {
                    result.push(WhereClause::DirectEquality {
                        param,
                        rhs: rhs_hir,
                    });
                }
            },
            WhereConstraint::NegativeBound { .. } => {
                // Negative bounds are not modeled in inference where clauses.
            },
        }
    }
    result
}

fn resolve_type_entity(
    ctx: &QueryContext<'_>,
    ast_ty: &AstType,
    entity: Entity,
    root: Entity,
) -> Option<Entity> {
    let AstType::Named { segments, .. } = ast_ty else {
        return None;
    };
    let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
    match ctx.query(ResolveTypePath {
        segments: seg_names,
        context: entity,
        root,
    }) {
        TypeResolution::Found(e) => Some(e),
        TypeResolution::SelfType => resolve_self_entity(ctx, entity, root),
        _ => None,
    }
}

/// Walk up from `start` to find the enclosing type entity that `Self` refers to.
fn resolve_self_entity(ctx: &QueryContext<'_>, start: Entity, root: Entity) -> Option<Entity> {
    let mut current = Some(start);
    while let Some(e) = current {
        match ctx.get::<NodeKind>(e) {
            Some(NodeKind::Extension) => {
                return ctx.query(kestrel_name_res::ExtensionTargetEntity { extension: e, root });
            },
            Some(NodeKind::Struct) | Some(NodeKind::Enum) | Some(NodeKind::Protocol) => {
                return Some(e);
            },
            _ => {},
        }
        current = ctx.parent_of(e);
    }
    None
}

/// `V = RHS` — resolve a bare type-param or associated-type LHS in `entity`'s
/// scope. Returns the resolved TypeParameter/TypeAlias entity.
fn resolve_type_param_or_assoc(
    ctx: &QueryContext<'_>,
    ast_ty: &AstType,
    entity: Entity,
    root: Entity,
) -> Option<Entity> {
    let AstType::Named { segments, .. } = ast_ty else {
        return None;
    };
    let all_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
    match ctx.query(ResolveTypePath {
        segments: all_names,
        context: entity,
        root,
    }) {
        TypeResolution::Found(e)
            if matches!(
                ctx.get::<NodeKind>(e),
                Some(&NodeKind::TypeParameter) | Some(&NodeKind::TypeAlias)
            ) =>
        {
            Some(e)
        },
        _ => None,
    }
}

/// `T.AssocName = RHS` — extract `(param_entity, assoc_name)`. `param_entity`
/// is resolved in `entity`'s scope (must be a type param or type alias).
fn extract_associated_type_path(
    ctx: &QueryContext<'_>,
    ast_ty: &AstType,
    entity: Entity,
    root: Entity,
) -> Option<(Entity, String)> {
    let AstType::Named { segments, .. } = ast_ty else {
        return None;
    };
    if segments.len() != 2 {
        return None;
    }
    let param_name = &segments[0].name;
    let assoc_name = &segments[1].name;
    match ctx.query(ResolveTypePath {
        segments: vec![param_name.clone()],
        context: entity,
        root,
    }) {
        TypeResolution::Found(e) => Some((e, assoc_name.clone())),
        _ => None,
    }
}

fn extract_protocol_type_args(
    ctx: &QueryContext<'_>,
    entity: Entity,
    root: Entity,
    protocol_ty: &AstType,
) -> Vec<HirTy> {
    match protocol_ty {
        AstType::Named { segments, .. } => segments
            .last()
            .map(|seg| {
                seg.type_args
                    .iter()
                    .map(|a| kestrel_hir_lower::lower_ast_type(ctx, entity, root, a))
                    .collect()
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}
