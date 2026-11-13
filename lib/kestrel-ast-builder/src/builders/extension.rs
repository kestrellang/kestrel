//! Extension declaration builder.

use kestrel_ast::AstType;
use kestrel_hecs::{Entity, World};
use kestrel_span::Span;

use kestrel_syntax_tree::SyntaxNodePtr;
use kestrel_syntax_tree::ast::{self, AstNode, HasName};
use kestrel_syntax_tree::utils::get_decl_span;

use super::helpers::*;
use super::type_param::build_type_parameters_except;
use crate::ast_type::lower_opt_type;
use crate::components::*;

/// Build an extension declaration entity from CST.
///
/// Components: NodeKind::Extension, FileId, ExtensionTarget,
/// [Conformances], [WhereClause], [Documentation], [Attributes],
/// [TypeParams] — the explicit `extend[T, …]` list, for params the target
/// doesn't bind (e.g. `extend[T] Int64: ArrayIndex[T]`).
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
    let self_alias = retarget_param_extension(world, entity, node);

    // Record what the LHS introduces before the pattern scan, which consumes
    // the same list. Both must agree on it — see `ExtensionLhsParamNames`.
    let lhs_names = collect_lhs_target_names(world, entity);
    world.set(entity, ExtensionLhsParamNames(lhs_names.clone()));

    // A declared param the target mentions (`extend[T] Box[T]`, `extend[X]
    // Box[W[X]]`) is the same param the undeclared spelling introduces: the
    // target binds it — a top-level arg binds the target's own param, a
    // nested one a pattern param solved from the receiver. Only params the
    // target never mentions (`extend[T] Int64: ArrayIndex[T]`) become the
    // extension's own `TypeParams`, which nothing at a use site could bind.
    let mut in_target: Vec<(String, Span)> = Vec::new();
    if let Some(target) = world.get::<ExtensionTarget>(entity) {
        for arg in target_args(&target.0) {
            collect_bare_names(arg, &mut in_target);
        }
    }
    let mut in_target: Vec<String> = in_target.into_iter().map(|(n, _)| n).collect();
    in_target.extend(self_alias);
    build_type_parameters_except(world, entity, node, file_entity, file_id, &in_target);
    let mut claimed = lhs_names;
    claimed.extend(declared_param_names(world, entity));
    introduce_pattern_candidates(world, entity, file_entity, &claimed);

    let members = node
        .extension_body()
        .map(|b| b.items().collect())
        .unwrap_or_default();
    (entity, members)
}

/// `extend[T, R] R: P[T] where R: Q` is the blanket conformance `extend[T]
/// Q: P[T]` spelled on a param. Retarget it at `Q` — the first protocol the
/// where clause bounds `R` by — so it takes the protocol-extension path, and
/// record `R` as this extension's name for `Self` (`ExtensionSelfAlias`). Its
/// remaining `R` clauses become `Self` clauses (`R: Copyable` →
/// `Self: Copyable`). Returns the alias, which is then no declared param.
/// A param target with no bound to retarget at is left as written (E452).
fn retarget_param_extension(
    world: &mut World,
    entity: Entity,
    node: &ast::ExtensionDeclaration,
) -> Option<String> {
    let target = world.get::<ExtensionTarget>(entity)?;
    let name = bare_name(&target.0)?.to_string();
    let declared = node
        .type_parameter_list()?
        .type_parameters()
        .filter_map(|p| p.name_text())
        .any(|n| n == name);
    if !declared {
        return None;
    }
    let mut clause = world.get::<WhereClause>(entity)?.clone();
    let idx = clause.0.iter().position(|c| {
        matches!(c, WhereConstraint::Bound { subject, protocols, .. }
            if bare_name(subject) == Some(name.as_str()) && !protocols.is_empty())
    })?;
    let WhereConstraint::Bound { protocols, .. } = &mut clause.0[idx] else {
        return None;
    };
    let new_target = protocols.remove(0);
    if protocols.is_empty() {
        clause.0.remove(idx);
    }
    for constraint in &mut clause.0 {
        let subject = match constraint {
            WhereConstraint::Bound { subject, .. }
            | WhereConstraint::NegativeBound { subject, .. } => subject,
            WhereConstraint::Equality { lhs, .. } => lhs,
        };
        rename_head(subject, &name, "Self");
    }
    world.set(entity, ExtensionTarget(new_target));
    if clause.0.is_empty() {
        world.remove_component::<WhereClause>(entity);
    } else {
        world.set(entity, clause);
    }
    world.set(entity, ExtensionSelfAlias(name.clone()));
    Some(name)
}

/// Rename a path's first segment (`R` / `R.Item` → `Self` / `Self.Item`).
fn rename_head(ty: &mut AstType, from: &str, to: &str) {
    if let AstType::Named { segments, .. } = ty
        && let Some(first) = segments.first_mut()
        && first.name == from
    {
        first.name = to.to_string();
    }
}

/// Names of the extension's declared `extend[T, …]` params.
fn declared_param_names(world: &World, entity: Entity) -> Vec<String> {
    let Some(params) = world.get::<TypeParams>(entity) else {
        return Vec::new();
    };
    params
        .0
        .iter()
        .filter_map(|&tp| world.get::<Name>(tp).map(|n| n.0.clone()))
        .collect()
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
) {
    let Some(target) = world.get::<ExtensionTarget>(entity) else {
        return;
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
        return;
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
        AstType::Named { segments, .. } => {
            segments.iter().flat_map(|s| s.type_args.iter()).collect()
        },
        AstType::Tuple(elems, _) => elems.iter().collect(),
        AstType::Function {
            params,
            return_type,
            ..
        } => params
            .iter()
            .chain(std::iter::once(return_type.as_ref()))
            .collect(),
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
/// the POINTEE is the param position, so it returns `["T"]` — without this,
/// the pattern scan would introduce a shadowing candidate for the same name.
///
/// This is a superset of the names that actually *bind*: `extend Box[Payload]`
/// yields `["Payload"]` even though `Payload` is a concrete type. Consumers
/// that need the binding parameters intersect this with the target nominal's
/// declared parameter names (see `ExtensionLhsParams` in kestrel-name-res);
/// the pattern scan wants the superset, since a name already mentioned on the
/// LHS must never be re-introduced.
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
