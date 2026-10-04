//! Extension declaration builder.

use std::collections::HashSet;

use kestrel_ast::{AstType, PathSegment};
use kestrel_hecs::{Entity, World};
use kestrel_span::Span;
use kestrel_syntax_tree::SyntaxNode;
use kestrel_syntax_tree::ast::{self, AstNode};
use kestrel_syntax_tree::utils::get_decl_span;

use super::helpers::*;
use crate::ast_type::lower_opt_type;
use crate::components::*;

/// Build an extension declaration entity from CST.
///
/// Components: NodeKind::Extension, FileId, ExtensionTarget,
/// [Conformances], [WhereClause], [Documentation],
/// [TypeParams] — when the RHS conformance list introduces free type
/// parameters that aren't bound by the extension target's own LHS args
/// (e.g. `extend Int64: ArrayIndex[T]` → free `T`).
///
/// Extensions have no Name — they extend an existing type — and take no
/// attributes (the grammar has none on `extend`).
pub fn build_extension(
    world: &mut World,
    node: &ast::ExtensionDeclaration,
    parent: Entity,
    file_entity: Entity,
    file_id: usize,
) -> (Entity, Vec<ast::Item>) {
    let syntax = node.syntax();
    let entity = world.spawn();

    world.set(entity, NodeKind::Extension);
    world.set(entity, FileId(file_entity));
    world.set(entity, DeclSpan(get_decl_span(syntax, file_id)));
    world.set(entity, CstNode(syntax.clone()));
    world.set_parent(entity, parent);

    if let Some(target_ty) = lower_opt_type(node.ty(), file_id) {
        world.set(entity, ExtensionTarget(target_ty));
    }

    set_documentation(world, entity, syntax);
    set_conformances(world, entity, node, file_id);
    set_where_clause(world, entity, node, file_id);

    // Record what the LHS introduces before the RHS scan, which consumes the
    // same list. Both must agree on it — see `ExtensionLhsParamNames`.
    let lhs_names = collect_lhs_target_names(world, entity);
    world.set(entity, ExtensionLhsParamNames(lhs_names.clone()));

    introduce_rhs_free_type_params(world, entity, file_entity, file_id, lhs_names);

    let members = node
        .extension_body()
        .map(|b| b.items().collect())
        .unwrap_or_default();
    (entity, members)
}

/// Scan the conformance RHS for free type parameters not already in scope
/// from the extension target's LHS, and register them as TypeParam entities
/// owned by this extension.
///
/// Why: `extend Int64: ArrayIndex[T]` has `T` only on the protocol RHS —
/// nothing on the LHS introduces it. Without this pass, `T` resolves to
/// nothing and the body fails with "cannot find type 'T'". We collect any
/// single-uppercase-letter named type appearing as a top-level argument of
/// a conformance protocol that isn't already bound by the LHS.
///
/// Limitations:
/// - Only top-level args are scanned. `extend Int64: Foo[Box[T]]` won't
///   auto-introduce `T` — use a top-level position.
/// - Only single-uppercase-letter identifiers count (`T`, `U`, `E`, `K`,
///   `V`). Names like `Self`, `Int64`, or `String` are intentionally
///   excluded — they're either reserved (Self) or real types that the
///   user clearly meant to reference, not introduce.
fn introduce_rhs_free_type_params(
    world: &mut World,
    entity: Entity,
    file_entity: Entity,
    file_id: usize,
    lhs_names: Vec<String>,
) {
    let conformances = match world.get::<Conformances>(entity) {
        Some(c) => c.0.clone(),
        None => return,
    };

    let mut seen: HashSet<String> = lhs_names.into_iter().collect();
    let mut new_params: Vec<(String, Span, SyntaxNode)> = Vec::new();

    let cst = match world.get::<CstNode>(entity) {
        Some(node) => node.0.clone(),
        None => return,
    };

    for item in &conformances {
        let proto_ty = match item {
            ConformanceItem::Positive(ty, _) => ty,
            ConformanceItem::Negative(ty, _) => ty,
        };
        let AstType::Named { segments, .. } = proto_ty else {
            continue;
        };
        let Some(last) = segments.last() else {
            continue;
        };
        for arg in &last.type_args {
            let AstType::Named {
                segments: arg_segs,
                span,
            } = arg
            else {
                continue;
            };
            if arg_segs.len() != 1 {
                continue;
            }
            let seg: &PathSegment = &arg_segs[0];
            if !seg.type_args.is_empty() {
                continue;
            }
            if !is_free_type_param_name(&seg.name) {
                continue;
            }
            if seen.contains(&seg.name) {
                continue;
            }
            seen.insert(seg.name.clone());
            new_params.push((seg.name.clone(), span.clone(), cst.clone()));
        }
    }

    if new_params.is_empty() {
        return;
    }

    let _ = file_id;
    let mut type_param_entities: Vec<Entity> = Vec::new();
    for (name, span, cst_ref) in new_params {
        let tp = world.spawn();
        world.set(tp, NodeKind::TypeParameter);
        world.set(tp, Name(name));
        world.set(tp, FileId(file_entity));
        world.set(tp, DeclSpan(span));
        world.set(tp, CstNode(cst_ref));
        world.set_parent(tp, entity);
        type_param_entities.push(tp);
    }

    let combined: Vec<Entity> = match world.get::<TypeParams>(entity) {
        Some(existing) => {
            let mut out = existing.0.clone();
            out.extend(type_param_entities);
            out
        },
        None => type_param_entities,
    };
    world.set(entity, TypeParams(combined));
}

/// True for identifiers that look like free type-param introductions —
/// single uppercase ASCII letters (`T`, `U`, `E`, `K`, `V`, …). Avoids
/// accidentally introducing a free param for `Self` or for real types
/// that happen to be in scope.
fn is_free_type_param_name(name: &str) -> bool {
    let mut chars = name.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => c.is_ascii_uppercase(),
        _ => false,
    }
}

/// Collect single-segment names appearing as top-level type args of the
/// extension's target, in source order. For `extend Pair[T, U]` returns
/// `["T", "U"]`. For `extend Int64` returns `[]`. For a ref target `extend &T`
/// the POINTEE is the param position, so it returns `["T"]` — without this, an
/// RHS scan would introduce a shadowing free param for the same name.
///
/// This is a superset of the names that actually *bind*: `extend Box[Payload]`
/// yields `["Payload"]` even though `Payload` is a concrete type. Consumers
/// that need the binding parameters intersect this with the target nominal's
/// declared parameter names (see `ExtensionLhsParams` in kestrel-name-res);
/// the RHS free-param scan wants the superset, since a name already mentioned
/// on the LHS must never be re-introduced.
fn collect_lhs_target_names(world: &World, entity: Entity) -> Vec<String> {
    let mut names = Vec::new();
    let mut push = |name: &str| {
        if !names.iter().any(|n: &String| n == name) {
            names.push(name.to_string());
        }
    };
    let Some(target) = world.get::<ExtensionTarget>(entity) else {
        return names;
    };
    if let AstType::Ref { inner, .. } = &target.0 {
        if let AstType::Named { segments, .. } = inner.as_ref()
            && segments.len() == 1
            && segments[0].type_args.is_empty()
        {
            push(&segments[0].name);
        }
        return names;
    }
    let AstType::Named { segments, .. } = &target.0 else {
        return names;
    };
    let Some(last) = segments.last() else {
        return names;
    };
    for arg in &last.type_args {
        if let AstType::Named { segments: segs, .. } = arg
            && segs.len() == 1
            && segs[0].type_args.is_empty()
        {
            push(&segs[0].name);
        }
    }
    names
}
