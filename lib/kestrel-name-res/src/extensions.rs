//! Extension resolution queries.
//!
//! Finds extensions for a given type entity and resolves extension
//! target types from AstType to entities.

use kestrel_ast_builder::{
    ExtensionLhsParamNames, ExtensionPatternCandidates, ExtensionTarget, LangModule, LangType,
    Name, NodeKind, TypeParams,
};
use kestrel_hecs::{Entity, QueryContext, QueryFn};

use crate::resolve_type::{ResolveTypePath, TypeResolution};

// ===== ResolvedExtensionTarget =====

/// Component: resolved extension target entity.
///
/// Can be set during a mutation-phase pass after AST building, or
/// computed lazily by ExtensionTargetEntity query.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ResolvedExtensionTarget(pub Entity);

// ===== ExtensionTargetEntity =====

/// Query: resolve an extension's target AstType to a type entity.
///
/// Reads the ExtensionTarget(AstType) component and resolves it
/// via ResolveTypePath. No cycle risk because type resolution never
/// looks through extensions (extensions lack the Typed marker).
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ExtensionTargetEntity {
    pub extension: Entity,
    pub root: Entity,
}

impl QueryFn for ExtensionTargetEntity {
    type Output = Option<Entity>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Option<Entity> {
        // Check for pre-resolved target first
        if let Some(resolved) = ctx.get::<ResolvedExtensionTarget>(self.extension) {
            return Some(resolved.0);
        }

        // Get the unresolved AstType from the ExtensionTarget component
        let target = ctx.get::<ExtensionTarget>(self.extension)?;
        let ast_type = &target.0;

        // Structural singletons `()` and `!` resolve to synthetic `lang` entities
        // (named "()" / "!") so they can be extension targets. Reference types
        // `&T` / `&mutating T` resolve to the generic synthetic entities
        // `lang.&` / `lang.&mutating` (pointee = the single type arg).
        match ast_type {
            kestrel_ast::AstType::Unit(..) => {
                return resolve_lang_type(ctx, self.root, LangType::Unit);
            },
            kestrel_ast::AstType::Never(..) => {
                return resolve_lang_type(ctx, self.root, LangType::Never);
            },
            kestrel_ast::AstType::Ref { mutating, .. } => {
                let ty = LangType::Ref {
                    mutating: *mutating,
                };
                return resolve_lang_type(ctx, self.root, ty);
            },
            // `(X, Y)` → the synthetic `lang.(,)` of that arity.
            kestrel_ast::AstType::Tuple(elems, _) if elems.len() >= 2 => {
                return resolve_lang_type(ctx, self.root, LangType::Tuple(elems.len()));
            },
            // Type sugar names its standard type: `extend [T]` is
            // `extend Array[T]` (and likewise `T?`, `[K: V]`).
            kestrel_ast::AstType::Array(..)
            | kestrel_ast::AstType::Optional(..)
            | kestrel_ast::AstType::Dictionary(..) => {
                let (builtin, _) = sugar_builtin(ast_type)?;
                return crate::resolve_sugar_type(ctx, self.root, builtin);
            },
            _ => {},
        }

        // Extract path segments from the AstType
        let kestrel_ast::AstType::Named { segments, .. } = ast_type else {
            return None;
        };

        let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();

        // Resolve using the extension's parent as context (the module it's in)
        let context = ctx.parent_of(self.extension).unwrap_or(self.root);

        let result = ctx.query(ResolveTypePath {
            segments: seg_names,
            context,
            root: self.root,
        });

        match result {
            TypeResolution::Found(entity) => Some(entity),
            _ => None,
        }
    }
}

// ===== ExtensionLhsParams =====

/// Query: the target type parameters an extension's LHS actually *binds*.
///
/// THE answer to "which type parameters does `extend Foo[…]` bring into
/// scope?", for every consumer. `extend Box[T]` binds Box's `T`;
/// `extend Box[Payload]` binds nothing, because `Payload` names no parameter
/// of Box. The returned entities belong to the target nominal — an extension
/// never carries `TypeParams` for its LHS.
///
/// Both halves matter. Taking the target's parameters unfiltered leaks `T`
/// into `extend Box[Payload]` bodies; ignoring them entirely (the old
/// `TypeParams`-only view) makes E439's shadowing walk blind to every generic
/// extension.
///
/// Each entry pairs the name the extension WROTE with the parameter it binds;
/// consumers match on that name, never the parameter's own `Name`:
/// - a nominal target binds by name, as it always has (`extend Box[U]` binds
///   nothing);
/// - a structural target (`&T`, `(X, Y)`) binds by position — its synthetic
///   `lang` entity's parameter names are invented. An element binds only when
///   it is a bare name that resolves to no type, so `extend (Int64, Y)` keeps
///   `Int64` concrete and binds `Y` alone;
/// - a NESTED position (`X` in `extend Box[W[X]]`) binds an extension-owned
///   pattern parameter (`ExtensionPatternParams`), solved by matching the
///   receiver's arg against the written pattern.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ExtensionLhsParams {
    pub extension: Entity,
    pub root: Entity,
}

impl QueryFn for ExtensionLhsParams {
    type Output = Vec<(String, Entity)>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Vec<(String, Entity)> {
        let mut out = self.target_params(ctx);
        out.extend(ctx.query(ExtensionPatternParams {
            extension: self.extension,
            root: self.root,
        }));
        out
    }
}

impl ExtensionLhsParams {
    /// The target nominal's parameters the top-level args bind.
    fn target_params(&self, ctx: &QueryContext<'_>) -> Vec<(String, Entity)> {
        let Some(lhs) = ctx.get::<ExtensionLhsParamNames>(self.extension) else {
            return Vec::new();
        };
        if lhs.0.is_empty() {
            return Vec::new();
        }
        let Some(target) = ctx.query(ExtensionTargetEntity {
            extension: self.extension,
            root: self.root,
        }) else {
            return Vec::new();
        };
        let Some(params) = ctx.get::<TypeParams>(target) else {
            return Vec::new();
        };
        if let Some(elements) = structural_target_elements(ctx, self.extension, target) {
            return self.bind_positionally(ctx, &elements, &params.0);
        }
        params
            .0
            .iter()
            .filter_map(|&tp| {
                let name = &ctx.get::<Name>(tp)?.0;
                lhs.0.contains(name).then(|| (name.clone(), tp))
            })
            .collect()
    }

    /// Pair each bare, unresolvable element name with the parameter at its
    /// position.
    fn bind_positionally(
        &self,
        ctx: &QueryContext<'_>,
        elements: &[kestrel_ast::AstType],
        params: &[Entity],
    ) -> Vec<(String, Entity)> {
        let context = ctx.parent_of(self.extension).unwrap_or(self.root);
        elements
            .iter()
            .zip(params)
            .filter_map(|(element, &param)| {
                let kestrel_ast::AstType::Named { segments, .. } = element else {
                    return None;
                };
                let [segment] = segments.as_slice() else {
                    return None;
                };
                if !segment.type_args.is_empty() {
                    return None;
                }
                names_no_type(ctx, context, self.root, &segment.name)
                    .then(|| (segment.name.clone(), param))
            })
            .collect()
    }
}

/// THE rule for whether a name written in an extension target introduces a
/// parameter: it resolves to no type from the extension's enclosing scope.
fn names_no_type(ctx: &QueryContext<'_>, context: Entity, root: Entity, name: &str) -> bool {
    let resolved = ctx.query(ResolveTypePath {
        segments: vec![name.to_string()],
        context,
        root,
    });
    matches!(resolved, TypeResolution::NotFound(..))
}

// ===== ExtensionPatternParams =====

/// Query: the pattern parameters an extension's target introduces at NESTED
/// positions — `X` in `extend Box[W[X]]`, `A`/`B` in `extend Box[(A, B)]` —
/// as `(written name, parameter)`. The builder records every nested bare name
/// as a candidate (`ExtensionPatternCandidates`); a candidate is a parameter
/// iff it names no type, so `extend Box[W[Int64]]` introduces nothing.
///
/// The parameters belong to the extension. Consumers that need the
/// extension's own generic parameters in full use `ExtensionOwnTypeParams`.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ExtensionPatternParams {
    pub extension: Entity,
    pub root: Entity,
}

impl QueryFn for ExtensionPatternParams {
    type Output = Vec<(String, Entity)>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Vec<(String, Entity)> {
        let Some(candidates) = ctx.get::<ExtensionPatternCandidates>(self.extension) else {
            return Vec::new();
        };
        let context = ctx.parent_of(self.extension).unwrap_or(self.root);
        candidates
            .0
            .iter()
            .filter_map(|&tp| {
                let name = ctx.get::<Name>(tp)?.0.clone();
                names_no_type(ctx, context, self.root, &name).then_some((name, tp))
            })
            .collect()
    }
}

// ===== ExtensionOwnTypeParams =====

/// Query: the generic parameters an extension itself owns, in order — its
/// free RHS parameters (`TypeParams`, e.g. `T` in `extend Int64:
/// ArrayIndex[T]`), then its pattern parameters. The target nominal's own
/// parameters are NOT included. THE list every layer that instantiates an
/// extension member (MIR signatures, call-site type args) must agree on.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ExtensionOwnTypeParams {
    pub extension: Entity,
    pub root: Entity,
}

impl QueryFn for ExtensionOwnTypeParams {
    type Output = Vec<Entity>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Vec<Entity> {
        let mut out: Vec<Entity> = ctx
            .get::<TypeParams>(self.extension)
            .map(|tp| tp.0.clone())
            .unwrap_or_default();
        out.extend(
            ctx.query(ExtensionPatternParams {
                extension: self.extension,
                root: self.root,
            })
            .into_iter()
            .map(|(_, tp)| tp),
        );
        out
    }
}

/// The written element types of a structural extension target — a ref's
/// pointee, a tuple's elements — when `target` is its synthetic `lang` entity.
fn structural_target_elements(
    ctx: &QueryContext<'_>,
    extension: Entity,
    target: Entity,
) -> Option<Vec<kestrel_ast::AstType>> {
    if lang_ref_mutability(ctx, target).is_none() && lang_tuple_arity(ctx, target).is_none() {
        return None;
    }
    match &ctx.get::<ExtensionTarget>(extension)?.0 {
        kestrel_ast::AstType::Ref { inner, .. } => Some(vec![inner.as_ref().clone()]),
        kestrel_ast::AstType::Tuple(elems, _) => Some(elems.clone()),
        _ => None,
    }
}

/// Reverse detector for the synthetic tuple entities: the arity of
/// `lang.(,)`-style entities, `None` otherwise. THE check for every
/// Entity-keyed site that must map the entity back to a tuple type.
pub fn lang_tuple_arity(ctx: &QueryContext<'_>, entity: Entity) -> Option<usize> {
    match ctx.get::<LangType>(entity)? {
        LangType::Tuple(arity) => Some(*arity),
        _ => None,
    }
}

/// Reverse detector for the synthetic ref-extension entities: `Some(false)`
/// for `lang.&`, `Some(true)` for `lang.&mutating`, `None` otherwise. THE
/// shared check for every Entity-keyed site that must map the entity back to
/// a ref type (the type layer never uses these entities).
pub fn lang_ref_mutability(ctx: &QueryContext<'_>, entity: Entity) -> Option<bool> {
    match ctx.get::<LangType>(entity)? {
        LangType::Ref { mutating } => Some(*mutating),
        _ => None,
    }
}

/// The seeded `lang` entity for `ty` (the structural singletons `()` / `!`,
/// the ref and tuple extension targets, the primitives), found by its
/// [`LangType`] tag under the [`LangModule`]-marked module.
pub fn resolve_lang_type(ctx: &QueryContext<'_>, root: Entity, ty: LangType) -> Option<Entity> {
    let lang = ctx
        .children_of(root)
        .iter()
        .copied()
        .find(|&c| ctx.get::<LangModule>(c).is_some())?;
    ctx.children_of(lang)
        .iter()
        .copied()
        .find(|&c| ctx.get::<LangType>(c) == Some(&ty))
}

// ===== ExtensionsFor =====

/// Query: find all extensions targeting a given type entity.
///
/// Walks the entire module hierarchy from root, collecting all
/// Extension entities whose resolved target matches the given type.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ExtensionsFor {
    pub target: Entity,
    pub root: Entity,
}

impl QueryFn for ExtensionsFor {
    type Output = Vec<Entity>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Vec<Entity> {
        let mut extensions = Vec::new();
        collect_extensions(ctx, self.root, self.target, self.root, &mut extensions);
        extensions
    }
}

/// Recursively walk the hierarchy to find Extension entities targeting `target`.
fn collect_extensions(
    ctx: &QueryContext<'_>,
    current: Entity,
    target: Entity,
    root: Entity,
    out: &mut Vec<Entity>,
) {
    for &child in ctx.children_of(current) {
        let kind = ctx.get::<NodeKind>(child);
        match kind {
            Some(&NodeKind::Extension) => {
                // Resolve extension target and check if it matches
                let resolved = ctx.query(ExtensionTargetEntity {
                    extension: child,
                    root,
                });
                if resolved == Some(target) {
                    out.push(child);
                }
            },
            Some(&NodeKind::Module) => {
                // Recurse into modules
                collect_extensions(ctx, child, target, root, out);
            },
            _ => {},
        }
    }
}

/// The lang item and standard type name a piece of type sugar denotes.
pub fn sugar_builtin(ty: &kestrel_ast::AstType) -> Option<(kestrel_hir::Builtin, &'static str)> {
    use kestrel_hir::Builtin;
    Some(match ty {
        kestrel_ast::AstType::Array(..) => (Builtin::ArrayTypeOperator, "Array"),
        kestrel_ast::AstType::Optional(..) => (Builtin::OptionalTypeOperator, "Optional"),
        kestrel_ast::AstType::Dictionary(..) => (Builtin::DictionaryTypeOperator, "Dictionary"),
        kestrel_ast::AstType::Result { .. } => (Builtin::ResultTypeOperator, "Result"),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_ast::{AstType, PathSegment};
    use kestrel_ast_builder::ExtensionTarget;
    use kestrel_ast_builder::{Name, Typed, Vis};
    use kestrel_hecs::World;
    use kestrel_span::Span;

    fn test_span() -> Span {
        Span::synthetic(0)
    }

    /// Build: root > std > core > [Int64(pub, Typed)]
    ///                           > [extend Int64 { ... }]
    fn setup() -> (World, Entity, Entity) {
        let mut world = World::new();
        world.begin_revision();

        let root = world.spawn();
        world.set(root, NodeKind::Module);
        world.set(root, Name(Name::ROOT.into()));

        let std = world.spawn();
        world.set(std, NodeKind::Module);
        world.set(std, Name("std".into()));
        world.set(std, kestrel_ast_builder::StdRootModule);
        world.set_parent(std, root);

        let core = world.spawn();
        world.set(core, NodeKind::Module);
        world.set(core, Name("core".into()));
        world.set_parent(core, std);

        let int64 = world.spawn();
        world.set(int64, NodeKind::Struct);
        world.set(int64, Name("Int64".into()));
        world.set(int64, Vis::Public);
        world.set(int64, Typed);
        world.set_parent(int64, core);

        // Extension targeting Int64
        let ext = world.spawn();
        world.set(ext, NodeKind::Extension);
        world.set(
            ext,
            ExtensionTarget(AstType::Named {
                segments: vec![PathSegment {
                    name: "Int64".into(),
                    type_args: vec![],
                    span: test_span(),
                }],
                span: test_span(),
            }),
        );
        world.set_parent(ext, core);

        (world, root, int64)
    }

    #[test]
    fn extension_target_resolves() {
        let (world, root, int64) = setup();
        let ctx = world.query_context();

        // Find the extension entity
        let std = ctx
            .children_of(root)
            .iter()
            .find(|&&e| ctx.get::<Name>(e).is_some_and(|n| n.0 == "std"))
            .copied()
            .unwrap();
        let core = ctx
            .children_of(std)
            .iter()
            .find(|&&e| ctx.get::<Name>(e).is_some_and(|n| n.0 == "core"))
            .copied()
            .unwrap();
        let ext = ctx
            .children_of(core)
            .iter()
            .find(|&&e| ctx.get::<NodeKind>(e) == Some(&NodeKind::Extension))
            .copied()
            .unwrap();

        let resolved = ctx.query(ExtensionTargetEntity {
            extension: ext,
            root,
        });
        assert_eq!(resolved, Some(int64));
    }

    #[test]
    fn extensions_for_finds_matching() {
        let (world, root, int64) = setup();
        let ctx = world.query_context();

        let exts = ctx.query(ExtensionsFor {
            target: int64,
            root,
        });
        assert_eq!(exts.len(), 1);
    }

    #[test]
    fn extensions_for_no_match() {
        let (mut world, root, _) = setup();
        // Spawn a random struct — no extensions target it
        let other = world.spawn();
        world.set(other, NodeKind::Struct);
        world.set(other, Name("Other".into()));
        world.set(other, Typed);
        world.set_parent(other, root);

        let ctx = world.query_context();
        let exts = ctx.query(ExtensionsFor {
            target: other,
            root,
        });
        assert!(exts.is_empty());
    }
}
