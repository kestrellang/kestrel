//! Type lowering: AstType → HirTy.
//!
//! Resolves type paths to entities and expands sugar types
//! (Array, Optional, Dictionary, Result) into Named types.
//!
//! The standalone `lower_ast_type` function is the shared implementation
//! used both during body lowering (via LowerCtx) and by type inference
//! for declaration-level types (Callable params, TypeAnnotation, etc.).

use kestrel_ast::AstType;
use kestrel_ast_builder::{Callable, DeclSpan, ExtensionTarget, NodeKind, TypeAnnotation, TypeParams};
use kestrel_hecs::{Entity, QueryContext, QueryFn};
use kestrel_hir::ty::HirTy;
use kestrel_name_res::{ResolveTypePath, TypeResolution};
use kestrel_reporting2::{Diagnostic, Label};
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

                    // Validate type argument arity when explicit args are provided
                    if !args.is_empty() {
                        if let Some(tp) = ctx.get::<TypeParams>(entity) {
                            let total = tp.0.len();
                            let required = tp.0.iter()
                                .filter(|&&p| ctx.get::<TypeAnnotation>(p).is_none())
                                .count();
                            if args.len() < required {
                                let type_name = segments.iter().map(|s| s.name.as_str()).collect::<Vec<_>>().join(".");
                                ctx.accumulate(Diagnostic::error()
                                    .with_message(format!("too few type arguments for '{type_name}'"))
                                    .with_labels(vec![
                                        Label::primary(span.file_id, span.range())
                                            .with_message(format!("expected at least {required}, got {}", args.len())),
                                    ]));
                                return HirTy::Error(span.clone());
                            } else if args.len() > total {
                                let type_name = segments.iter().map(|s| s.name.as_str()).collect::<Vec<_>>().join(".");
                                ctx.accumulate(Diagnostic::error()
                                    .with_message(format!("too many type arguments for '{type_name}'"))
                                    .with_labels(vec![
                                        Label::primary(span.file_id, span.range())
                                            .with_message(format!("expected at most {total}, got {}", args.len())),
                                    ]));
                                return HirTy::Error(span.clone());
                            }
                        }
                    }

                    let args = fill_type_arg_defaults(ctx, root, entity, args);

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
                        ctx.accumulate(Diagnostic::error()
                            .with_message("'Self' is not valid in this scope")
                            .with_labels(vec![
                                Label::primary(span.file_id, span.range()),
                            ]));
                        HirTy::Error(span.clone())
                    }
                }
                TypeResolution::NotFound(ref seg) => {
                    let type_name = segments.iter().map(|s| s.name.as_str()).collect::<Vec<_>>().join(".");
                    ctx.accumulate(Diagnostic::error()
                        .with_message(format!("cannot find type '{type_name}' in this scope"))
                        .with_labels(vec![
                            Label::primary(span.file_id, span.range())
                                .with_message(format!("not found (failed at '{seg}')")),
                        ]));
                    HirTy::Error(span.clone())
                }
                TypeResolution::NotAType(entity) => {
                    let type_name = segments.iter().map(|s| s.name.as_str()).collect::<Vec<_>>().join(".");
                    let mut labels = vec![
                        Label::primary(span.file_id, span.range())
                            .with_message("expected a type"),
                    ];
                    if let Some(decl) = ctx.get::<DeclSpan>(entity) {
                        let kind = ctx.get::<NodeKind>(entity)
                            .map(|k| format!("{k:?}"))
                            .unwrap_or_else(|| "symbol".to_string());
                        labels.push(
                            Label::secondary(decl.0.file_id, decl.0.range())
                                .with_message(format!("'{type_name}' is a {kind}, not a type")),
                        );
                    }
                    ctx.accumulate(Diagnostic::error()
                        .with_message(format!("'{type_name}' is not a type"))
                        .with_labels(labels));
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
        let args = fill_type_arg_defaults(ctx, root, entity, lowered_args);
        HirTy::Named {
            entity,
            args,
            span: span.clone(),
        }
    } else {
        ctx.accumulate(Diagnostic::error()
            .with_message(format!("{name} is not defined"))
            .with_labels(vec![
                Label::primary(span.file_id, span.range()),
            ])
            .with_notes(vec!["is the standard library imported?".to_string()]));
        HirTy::Error(span.clone())
    }
}

/// Fill in default type arguments for type parameters beyond user-provided args.
/// Defaults are stored as `TypeAnnotation` on the type param entity and lowered in
/// the defining scope so e.g. `H = DefaultHasher` resolves in the declaring module.
/// Stops on the first type param without a default (defaults must be trailing).
fn fill_type_arg_defaults(
    ctx: &QueryContext<'_>,
    root: Entity,
    entity: Entity,
    mut args: Vec<HirTy>,
) -> Vec<HirTy> {
    let Some(tp) = ctx.get::<TypeParams>(entity) else {
        return args;
    };
    if args.len() >= tp.0.len() {
        return args;
    }
    let type_params = tp.0.clone();
    for &param in type_params.iter().skip(args.len()) {
        match ctx.query(LowerTypeAnnotation { entity: param, root }) {
            Some(default_ty) => args.push(default_ty),
            None => break,
        }
    }
    args
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

/// Query: lower an extension target's type arguments to HirTy.
///
/// For `extend Box[lang.i64]`, returns `Some(vec![HirTy for i64])`.
/// For `extend Box` (no type args), returns `Some(vec![])`.
/// Returns None if the entity has no ExtensionTarget component.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct LowerExtensionTargetTypeArgs {
    pub extension: Entity,
    pub root: Entity,
}

impl QueryFn for LowerExtensionTargetTypeArgs {
    type Output = Option<Vec<HirTy>>;

    fn describe(&self) -> String {
        format!("LowerExtensionTargetTypeArgs(ext={:?})", self.extension)
    }

    fn execute(&self, ctx: &QueryContext<'_>) -> Option<Vec<HirTy>> {
        let target = ctx.get::<ExtensionTarget>(self.extension)?;
        let AstType::Named { segments, .. } = &target.0 else {
            return Some(vec![]);
        };

        // Type args are on the last path segment (the type name)
        let Some(last_seg) = segments.last() else {
            return Some(vec![]);
        };

        if last_seg.type_args.is_empty() {
            return Some(vec![]);
        }

        // Lower each type arg in the extension's own scope (so type params like T are visible)
        let context = self.extension;
        let args = last_seg
            .type_args
            .iter()
            .map(|ast_ty| lower_ast_type(ctx, context, self.root, ast_ty))
            .collect();
        Some(args)
    }
}
