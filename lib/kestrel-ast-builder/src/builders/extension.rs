//! Extension declaration builder.

use std::collections::HashSet;

use kestrel_ast::{AstType, PathSegment};
use kestrel_hecs::{Entity, World};
use kestrel_span::Span;

use kestrel_syntax_tree::SyntaxNodePtr;
use kestrel_syntax_tree::ast::{self, AstNode};
use kestrel_syntax_tree::utils::get_decl_span;

use super::helpers::*;
use crate::ast_type::lower_opt_type;
use crate::components::*;

/// Build an extension declaration entity from CST.
///
/// Components: NodeKind::Extension, FileId, ExtensionTarget,
/// [Conformances], [WhereClause], [Documentation], [Attributes],
/// [TypeParams] — when the RHS conformance list introduces free type
/// parameters that aren't bound by the extension target's own LHS args
/// (e.g. `extend Int64: ArrayIndex[T]` → free `T`).
///
/// Extensions have no Name — they extend an existing type.
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
    world.set(entity, CstNode(SyntaxNodePtr::new(syntax)));
    world.set_parent(entity, parent);

    if let Some(target_ty) = lower_opt_type(node.ty(), file_id) {
        world.set(entity, ExtensionTarget(target_ty));
    }

    set_documentation(world, entity, syntax);
    set_attributes(world, entity, node, file_id);
    set_conformances(world, entity, node, file_id);
    set_where_clause(world, entity, node, file_id);

    // Record what the LHS introduces before the RHS scan, which consumes the
    // same list. Both must agree on it — see `ExtensionLhsParamNames`.
    let lhs_names = collect_lhs_target_names(world, entity);
    world.set(entity, ExtensionLhsParamNames(lhs_names.clone()));

    // Nested pattern names (`X` in `extend Box[W[X]]`) are claimed before the
    // RHS scan too, so `extend Box[(A, B)]: P[A]` doesn't introduce a second `A`.
    let pattern_names = introduce_pattern_candidates(world, entity, file_entity, &lhs_names);
    let mut claimed = lhs_names;
    claimed.extend(pattern_names);
    introduce_rhs_free_type_params(world, entity, file_entity, file_id, claimed);

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
    let mut new_params: Vec<(String, Span, SyntaxNodePtr)> = Vec::new();

    let cst = match world.get::<CstNode>(entity) {
        Some(node) => node.0,
        None => return,
    };

    for item in &conformances {
        let proto_ty = match item {
            ConformanceItem::Positive(ty, ..) => ty,
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
            new_params.push((seg.name.clone(), span.clone(), cst));
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

/// Spawn a candidate `TypeParameter` for every bare name at a NESTED position
/// of the target (`X` in `extend Box[W[X]]`, `A`/`B` in `extend Box[(A, B)]`),
/// recorded in `ExtensionPatternCandidates`, and return their names. Top-level
/// arg names bind target params instead (`collect_lhs_target_names`) and are
/// skipped; name resolution keeps only candidates that name no type.
fn introduce_pattern_candidates(
    world: &mut World,
    entity: Entity,
    file_entity: Entity,
    lhs_names: &[String],
) -> Vec<String> {
    let Some(target) = world.get::<ExtensionTarget>(entity) else {
        return Vec::new();
    };
    let mut found: Vec<(String, Span)> = Vec::new();
    for arg in target_args(&target.0) {
        // A bare top-level arg is a target-param position, not a pattern.
        if bare_name(arg).is_some() {
            continue;
        }
        collect_bare_names(arg, &mut found);
    }
    found.retain(|(name, _)| !lhs_names.contains(name));
    let mut names: Vec<String> = Vec::new();
    found.retain(|(name, _)| {
        let new = !names.contains(name);
        if new {
            names.push(name.clone());
        }
        new
    });
    if found.is_empty() {
        return names;
    }
    let cst = world.get::<CstNode>(entity).map(|c| c.0);
    let candidates: Vec<Entity> = found
        .into_iter()
        .map(|(name, span)| {
            let tp = world.spawn();
            world.set(tp, NodeKind::TypeParameter);
            world.set(tp, Name(name));
            world.set(tp, FileId(file_entity));
            world.set(tp, DeclSpan(span));
            if let Some(cst) = cst {
                world.set(tp, CstNode(cst));
            }
            world.set_parent(tp, entity);
            tp
        })
        .collect();
    world.set(entity, ExtensionPatternCandidates(candidates));
    names
}

/// The target's type-argument positions: `Box[T]`'s args, the sugar
/// spellings' elements, a ref's pointee, a tuple's elements.
fn target_args(target: &AstType) -> Vec<&AstType> {
    match target {
        AstType::Ref { inner, .. } => vec![inner.as_ref()],
        AstType::Tuple(elems, _) => elems.iter().collect(),
        AstType::Named { segments, .. } => match segments.last() {
            Some(last) => last.type_args.iter().collect(),
            None => Vec::new(),
        },
        AstType::Array(elem, _) | AstType::Optional(elem, _) => vec![elem.as_ref()],
        AstType::Dictionary(key, val, _) => vec![key.as_ref(), val.as_ref()],
        _ => Vec::new(),
    }
}

/// `Some(name)` when `ty` is a single bare name with no type args.
fn bare_name(ty: &AstType) -> Option<&str> {
    let AstType::Named { segments, .. } = ty else {
        return None;
    };
    match segments.as_slice() {
        [seg] if seg.type_args.is_empty() => Some(&seg.name),
        _ => None,
    }
}

/// Every bare name at any depth inside `ty` (a bare `ty` itself included).
fn collect_bare_names(ty: &AstType, out: &mut Vec<(String, Span)>) {
    if let (Some(name), AstType::Named { span, .. }) = (bare_name(ty), ty) {
        out.push((name.to_string(), span.clone()));
        return;
    }
    let children: Vec<&AstType> = match ty {
        AstType::Named { segments, .. } => segments.iter().flat_map(|s| s.type_args.iter()).collect(),
        AstType::Tuple(elems, _) => elems.iter().collect(),
        AstType::Function { params, return_type, .. } => {
            params.iter().chain(std::iter::once(return_type.as_ref())).collect()
        },
        AstType::Array(elem, _) | AstType::Optional(elem, _) => vec![elem.as_ref()],
        AstType::Dictionary(key, val, _) => vec![key.as_ref(), val.as_ref()],
        AstType::Result { ok, err, .. } => vec![ok.as_ref(), err.as_ref()],
        AstType::Ref { inner, .. } => vec![inner.as_ref()],
        _ => Vec::new(),
    };
    for child in children {
        collect_bare_names(child, out);
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
    for arg in target_args(&target.0) {
        if let Some(name) = bare_name(arg) {
            push(name);
        }
    }
    names
}
