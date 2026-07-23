//! Render a structured signature for any documented declaration entity.
//!
//! Builds the signature from AST components rather than slicing the
//! source — that way generic brackets, accessor blocks, and type-alias
//! RHS round-trip without depending on parser-folding quirks. Also
//! supports a "hide bind names" mode that drops a parameter's internal
//! name and shows just `label: Type` (or just `Type` for unlabeled).

use std::collections::HashMap;

use kestrel_ast_builder::{
    AstParam, AstType, Callable, Computed, ConformanceItem, Conformances, CstNode, ExtensionTarget,
    FieldMutability, InitEffect, IsIndirect, MutatingAccessor, Name, NodeKind, ReceiverKind,
    Static, TypeAnnotation, TypeParams, Vis, WhereClause, WhereConstraint,
};
use kestrel_hecs::{Entity, World};
use kestrel_syntax_tree::SyntaxKind;

#[derive(Clone, Copy)]
pub struct Options {
    /// When true, render parameters as `label: Type` (combined or labeled
    /// case) or just `Type` (unlabeled), dropping the internal binding
    /// name even when it differs from the label.
    pub hide_bind_names: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            hide_bind_names: true,
        }
    }
}

pub fn build(world: &World, entity: Entity, opts: Options) -> String {
    let Some(kind) = world.get::<NodeKind>(entity).cloned() else {
        return String::new();
    };
    match kind {
        NodeKind::Function => build_function(world, entity, opts),
        NodeKind::Initializer => build_initializer(world, entity, opts),
        NodeKind::Subscript => build_subscript(world, entity, opts),
        NodeKind::Field => build_field(world, entity),
        NodeKind::Struct => build_type_decl(world, entity, "struct"),
        NodeKind::Enum => build_enum(world, entity),
        NodeKind::EnumCase => build_enum_case(world, entity, opts),
        NodeKind::Protocol => build_type_decl(world, entity, "protocol"),
        NodeKind::Extension => build_extension(world, entity),
        NodeKind::TypeAlias => build_type_alias(world, entity),
        _ => String::new(),
    }
}

/// Visibility lookup with a CST fallback. The parser sometimes builds a
/// declaration without a `Visibility` child node even though the source
/// has `private`/`public`, in which case the `Vis` component is unset —
/// we rescan the CST tokens to recover the modifier so the docs filter
/// doesn't leak `private` items into the rendered output.
pub fn visibility(world: &World, entity: Entity) -> Option<&'static str> {
    if let Some(vis) = world.get::<Vis>(entity) {
        return Some(match vis {
            Vis::Public => "public",
            Vis::Private => "private",
            Vis::Internal => "internal",
            Vis::Fileprivate => "fileprivate",
        });
    }
    let cst = world.get::<CstNode>(entity)?;
    for tok in cst
        .0
        .descendants_with_tokens()
        .filter_map(|e| e.into_token())
    {
        match tok.kind() {
            SyntaxKind::Public => return Some("public"),
            SyntaxKind::Private => return Some("private"),
            SyntaxKind::Internal => return Some("internal"),
            SyntaxKind::Fileprivate => return Some("fileprivate"),
            // Stop scanning once we hit the actual declaration keyword —
            // anything past that belongs to the body, not the preamble.
            SyntaxKind::Func
            | SyntaxKind::Struct
            | SyntaxKind::Enum
            | SyntaxKind::Protocol
            | SyntaxKind::Init
            | SyntaxKind::Subscript
            | SyntaxKind::Type
            | SyntaxKind::Var
            | SyntaxKind::Let
            | SyntaxKind::Case => return None,
            _ => continue,
        }
    }
    None
}

pub fn is_private(world: &World, entity: Entity) -> bool {
    matches!(
        visibility(world, entity),
        Some("private") | Some("fileprivate")
    )
}

// ============================================================================
// Renderers per kind
// ============================================================================

fn build_function(world: &World, entity: Entity, opts: Options) -> String {
    // Re-sugar the `some P`-in-param desugar: synthetic `__opaque_N` type
    // params (and their where constraints) are folded back into the
    // parameter types so users never see the internal names.
    let opaque = collect_opaque_resugar(world, entity);
    let mut s = String::new();
    push_visibility(&mut s, world, entity);
    if world.get::<Static>(entity).is_some() {
        s.push_str("static ");
    }
    push_receiver(&mut s, world, entity);
    s.push_str("func ");
    s.push_str(&name_of(world, entity));
    s.push_str(&type_params_str_filtered(world, entity, &opaque));
    push_params(&mut s, world, entity, opts, &opaque);
    push_return_type(&mut s, world, entity);
    s.push_str(&where_clause_str_filtered(world, entity, &opaque));
    s
}

fn build_initializer(world: &World, entity: Entity, opts: Options) -> String {
    let opaque = collect_opaque_resugar(world, entity);
    let mut s = String::new();
    push_visibility(&mut s, world, entity);
    s.push_str("init");
    s.push_str(&type_params_str_filtered(world, entity, &opaque));
    push_params(&mut s, world, entity, opts, &opaque);
    // Failable / throwing marker: the builder encodes the effect both as
    // `InitEffect` and as the init's synthetic return type (`()?` /
    // `() throws E`) — render it back as source syntax.
    match world.get::<InitEffect>(entity) {
        Some(InitEffect::Failable) => s.push('?'),
        Some(InitEffect::Throwing) => {
            if let Some(TypeAnnotation(AstType::Result { err, .. })) =
                world.get::<TypeAnnotation>(entity)
            {
                s.push_str(&format!(" throws {}", ty(err)));
            } else {
                s.push_str(" throws");
            }
        },
        None => {},
    }
    s.push_str(&where_clause_str_filtered(world, entity, &opaque));
    s
}

fn build_subscript(world: &World, entity: Entity, opts: Options) -> String {
    let opaque = collect_opaque_resugar(world, entity);
    let mut s = String::new();
    push_visibility(&mut s, world, entity);
    s.push_str("subscript");
    s.push_str(&type_params_str_filtered(world, entity, &opaque));
    push_params(&mut s, world, entity, opts, &opaque);
    push_return_type(&mut s, world, entity);
    s.push_str(&where_clause_str_filtered(world, entity, &opaque));
    push_accessors(&mut s, world, entity);
    s
}

fn build_field(world: &World, entity: Entity) -> String {
    let mut s = String::new();
    push_visibility(&mut s, world, entity);
    if world.get::<Static>(entity).is_some() {
        s.push_str("static ");
    }
    let kw = match world.get::<FieldMutability>(entity) {
        Some(FieldMutability::Var) => "var",
        Some(FieldMutability::Let) => "let",
        None => "var",
    };
    s.push_str(kw);
    s.push(' ');
    s.push_str(&name_of(world, entity));
    if let Some(ann) = world.get::<TypeAnnotation>(entity) {
        s.push_str(": ");
        s.push_str(&ty(&ann.0));
    }
    if world.get::<Computed>(entity).is_some() {
        push_accessors(&mut s, world, entity);
    }
    s
}

fn build_type_decl(world: &World, entity: Entity, keyword: &str) -> String {
    // Conformances are intentionally omitted from the header — they're
    // surfaced separately as `Implements <protocol>` member groups so a
    // long protocol list doesn't crowd the type's own signature.
    let mut s = String::new();
    push_visibility(&mut s, world, entity);
    s.push_str(keyword);
    s.push(' ');
    s.push_str(&name_of(world, entity));
    s.push_str(&type_params_str(world, entity));
    s.push_str(&negative_conformances_str(world, entity));
    s.push_str(&where_clause_str(world, entity));
    if keyword == "struct" {
        s.push_str(" { /* private fields */ }");
    }
    s
}

/// Render a type's negative conformances (`: not Copyable`) inline in the
/// header. Positive conformances are surfaced as `Implements` member groups,
/// but negative ones have no group to land in and change the type's
/// copy/usage semantics, so the header is the only place they can appear.
fn negative_conformances_str(world: &World, entity: Entity) -> String {
    let Some(conformances) = world.get::<Conformances>(entity) else {
        return String::new();
    };
    let parts: Vec<String> = conformances
        .0
        .iter()
        .filter_map(|item| match item {
            ConformanceItem::Negative(t, _) => Some(format!("not {}", ty(t))),
            ConformanceItem::Positive(..) => None,
        })
        .collect();
    if parts.is_empty() {
        return String::new();
    }
    format!(": {}", parts.join(", "))
}

fn build_enum(world: &World, entity: Entity) -> String {
    let mut s = String::new();
    push_visibility(&mut s, world, entity);
    if world.get::<IsIndirect>(entity).is_some() {
        s.push_str("indirect ");
    }
    s.push_str("enum ");
    s.push_str(&name_of(world, entity));
    s.push_str(&type_params_str(world, entity));
    s.push_str(&negative_conformances_str(world, entity));
    s.push_str(&where_clause_str(world, entity));
    s
}

fn build_enum_case(world: &World, entity: Entity, opts: Options) -> String {
    let mut s = String::new();
    s.push_str("case ");
    s.push_str(&name_of(world, entity));
    if world.get::<Callable>(entity).is_some() {
        push_params(&mut s, world, entity, opts, &OpaqueMap::new());
    }
    s
}

fn build_extension(world: &World, entity: Entity) -> String {
    let mut s = String::new();
    push_visibility(&mut s, world, entity);
    s.push_str("extension ");
    if let Some(target) = world.get::<ExtensionTarget>(entity) {
        s.push_str(&ty(&target.0));
    }
    s.push_str(&where_clause_str(world, entity));
    s
}

fn build_type_alias(world: &World, entity: Entity) -> String {
    let mut s = String::new();
    push_visibility(&mut s, world, entity);
    s.push_str("type ");
    s.push_str(&name_of(world, entity));
    s.push_str(&type_params_str(world, entity));
    if let Some(ann) = world.get::<TypeAnnotation>(entity) {
        s.push_str(" = ");
        s.push_str(&ty(&ann.0));
    }
    s
}

// ============================================================================
// Building blocks
// ============================================================================

fn push_visibility(s: &mut String, world: &World, entity: Entity) {
    if let Some(v) = visibility(world, entity) {
        s.push_str(v);
        s.push(' ');
    }
}

fn push_receiver(s: &mut String, world: &World, entity: Entity) {
    let Some(callable) = world.get::<Callable>(entity) else {
        return;
    };
    match callable.receiver {
        Some(ReceiverKind::Mutating) => s.push_str("mutating "),
        Some(ReceiverKind::Consuming) => s.push_str("consuming "),
        _ => {},
    }
}

fn name_of(world: &World, entity: Entity) -> String {
    world
        .get::<Name>(entity)
        .map(|n| n.0.clone())
        .unwrap_or_default()
}

/// Bounds recovered for one synthetic `__opaque_N` type parameter, keyed
/// by its name: the positive protocol bounds plus an optional negative
/// (`not Copyable`) bound. Used to render the param back as `some P`.
#[derive(Default)]
struct OpaqueBounds {
    bounds: Vec<AstType>,
    negative: Option<AstType>,
}

type OpaqueMap = HashMap<String, OpaqueBounds>;

/// Invert `desugar_opaque_params`: find synthetic `__opaque_N` type
/// params and pull their bounds out of the where clause so params can be
/// rendered as the `some P [and not C]` the user actually wrote.
fn collect_opaque_resugar(world: &World, entity: Entity) -> OpaqueMap {
    let mut map = OpaqueMap::new();
    let Some(params) = world.get::<TypeParams>(entity) else {
        return map;
    };
    for &tp in &params.0 {
        let name = name_of(world, tp);
        if name.starts_with("__opaque_") {
            map.insert(name, OpaqueBounds::default());
        }
    }
    if map.is_empty() {
        return map;
    }
    if let Some(wc) = world.get::<WhereClause>(entity) {
        for c in &wc.0 {
            match c {
                WhereConstraint::Bound {
                    subject, protocols, ..
                } => {
                    if let Some(n) = single_segment_name(subject)
                        && let Some(info) = map.get_mut(n)
                    {
                        info.bounds.extend(protocols.iter().cloned());
                    }
                },
                WhereConstraint::NegativeBound {
                    subject, protocol, ..
                } => {
                    if let Some(n) = single_segment_name(subject)
                        && let Some(info) = map.get_mut(n)
                    {
                        info.negative = Some(protocol.clone());
                    }
                },
                WhereConstraint::Equality { .. } => {},
            }
        }
    }
    map
}

/// `Foo` (single bare path segment, no type args) → `Some("Foo")`.
fn single_segment_name(t: &AstType) -> Option<&str> {
    let AstType::Named { segments, .. } = t else {
        return None;
    };
    let [seg] = segments.as_slice() else {
        return None;
    };
    seg.type_args.is_empty().then_some(seg.name.as_str())
}

/// Is this where-clause constraint about a synthetic opaque param (and
/// therefore folded into the param type rather than shown)?
fn constrains_opaque(c: &WhereConstraint, opaque: &OpaqueMap) -> bool {
    let subject = match c {
        WhereConstraint::Bound { subject, .. } => subject,
        WhereConstraint::NegativeBound { subject, .. } => subject,
        WhereConstraint::Equality { lhs, .. } => lhs,
    };
    single_segment_name(subject).is_some_and(|n| opaque.contains_key(n))
}

fn push_params(s: &mut String, world: &World, entity: Entity, opts: Options, opaque: &OpaqueMap) {
    s.push('(');
    if let Some(callable) = world.get::<Callable>(entity) {
        let parts: Vec<String> = callable
            .params
            .iter()
            .map(|p| param_str(p, opts.hide_bind_names, opaque))
            .collect();
        s.push_str(&parts.join(", "));
    }
    s.push(')');
}

fn param_str(p: &AstParam, hide_bind: bool, opaque: &OpaqueMap) -> String {
    let ty_str = p
        .ty
        .as_ref()
        .map(|t| param_ty(t, opaque))
        .unwrap_or_else(|| "_".into());
    // `is_mut` on AstParam encodes the `mutating` (or `consuming`)
    // access-mode keyword from the source. Render it back as written —
    // `mut` is Rust syntax, not Kestrel.
    let mods = if p.is_consuming {
        "consuming "
    } else if p.is_mut {
        "mutating "
    } else {
        ""
    };

    if hide_bind {
        match &p.label {
            Some(label) => format!("{}: {}{}", label, mods, ty_str),
            None => format!("{}{}", mods, ty_str),
        }
    } else {
        match (&p.label, p.name.as_str()) {
            (Some(label), name) if label == name => {
                format!("{}: {}{}", label, mods, ty_str)
            },
            (Some(label), name) => format!("{} {}: {}{}", label, name, mods, ty_str),
            (None, name) => format!("_ {}: {}{}", name, mods, ty_str),
        }
    }
}

fn push_return_type(s: &mut String, world: &World, entity: Entity) {
    if let Some(ann) = world.get::<TypeAnnotation>(entity) {
        s.push_str(" -> ");
        s.push_str(&ty(&ann.0));
    }
}

fn push_accessors(s: &mut String, world: &World, entity: Entity) {
    let mut has_setter = false;
    let mut has_ref = false;
    let mut has_mutating_ref = false;
    for &c in world.children_of(entity) {
        match world.get::<NodeKind>(c) {
            Some(NodeKind::Setter) => has_setter = true,
            Some(NodeKind::RefAccessor) => {
                if world.get::<MutatingAccessor>(c).is_some() {
                    has_mutating_ref = true;
                } else {
                    has_ref = true;
                }
            },
            _ => {},
        }
    }
    // Canonical provider order: a `ref` accessor replaces the `get` seat
    // (at most one read + one write provider per member).
    let mut parts: Vec<&str> = Vec::new();
    parts.push(if has_ref { "ref" } else { "get" });
    if has_setter {
        parts.push("set");
    }
    if has_mutating_ref {
        parts.push("mutating ref");
    }
    s.push_str(" { ");
    s.push_str(&parts.join(" "));
    s.push_str(" }");
}

/// Render a param type, substituting a synthetic opaque type-param
/// reference back to its `some P [and not C]` surface form.
fn param_ty(t: &AstType, opaque: &OpaqueMap) -> String {
    if let Some(info) = single_segment_name(t).and_then(|n| opaque.get(n)) {
        let b: Vec<_> = info.bounds.iter().map(ty).collect();
        let head = if b.is_empty() { "_".into() } else { b.join(" and ") };
        return match &info.negative {
            Some(neg) => format!("some {} and not {}", head, ty(neg)),
            None => format!("some {}", head),
        };
    }
    ty(t)
}

fn type_params_str(world: &World, entity: Entity) -> String {
    type_params_str_filtered(world, entity, &OpaqueMap::new())
}

/// Like [`type_params_str`] but omits synthetic `__opaque_N` params.
fn type_params_str_filtered(world: &World, entity: Entity, opaque: &OpaqueMap) -> String {
    let Some(params) = world.get::<TypeParams>(entity) else {
        return String::new();
    };
    let parts: Vec<String> = params
        .0
        .iter()
        .filter(|&&e| !opaque.contains_key(name_of(world, e).as_str()))
        .map(|&e| {
            let name = name_of(world, e);
            let default = world
                .get::<TypeAnnotation>(e)
                .map(|t| format!(" = {}", ty(&t.0)))
                .unwrap_or_default();
            format!("{}{}", name, default)
        })
        .collect();
    if parts.is_empty() {
        return String::new();
    }
    format!("[{}]", parts.join(", "))
}

pub(crate) fn where_clause_str(world: &World, entity: Entity) -> String {
    where_clause_str_filtered(world, entity, &OpaqueMap::new())
}

/// Like [`where_clause_str`] but omits constraints on synthetic
/// `__opaque_N` params (those render inline as `some P` instead).
fn where_clause_str_filtered(world: &World, entity: Entity, opaque: &OpaqueMap) -> String {
    let Some(wc) = world.get::<WhereClause>(entity) else {
        return String::new();
    };
    let parts: Vec<String> =
        wc.0.iter()
            .filter(|c| !constrains_opaque(c, opaque))
            .map(|c| match c {
                WhereConstraint::Bound {
                    subject, protocols, ..
                } => {
                    let ps: Vec<_> = protocols.iter().map(ty).collect();
                    format!("{}: {}", ty(subject), ps.join(" + "))
                },
                WhereConstraint::Equality { lhs, rhs, .. } => {
                    format!("{} == {}", ty(lhs), ty(rhs))
                },
                WhereConstraint::NegativeBound {
                    subject, protocol, ..
                } => format!("{}: not {}", ty(subject), ty(protocol)),
            })
            .collect();
    if parts.is_empty() {
        return String::new();
    }
    format!(" where {}", parts.join(", "))
}

/// Render an `AstType` as readable Kestrel syntax. Mirrors
/// `kestrel_ast::pretty::format_type` (which is private to that crate).
fn ty(t: &AstType) -> String {
    match t {
        AstType::Named { segments, .. } => segments
            .iter()
            .map(|s| {
                if s.type_args.is_empty() {
                    s.name.clone()
                } else {
                    let args: Vec<_> = s.type_args.iter().map(ty).collect();
                    format!("{}[{}]", s.name, args.join(", "))
                }
            })
            .collect::<Vec<_>>()
            .join("."),
        AstType::Tuple(elems, _) => {
            let inner: Vec<_> = elems.iter().map(ty).collect();
            format!("({})", inner.join(", "))
        },
        AstType::Function {
            params,
            return_type,
            ..
        } => {
            let p: Vec<_> = params.iter().map(ty).collect();
            format!("({}) -> {}", p.join(", "), ty(return_type))
        },
        AstType::Array(inner, _) => format!("[{}]", ty(inner)),
        AstType::Dictionary(k, v, _) => format!("[{}: {}]", ty(k), ty(v)),
        AstType::Optional(inner, _) => format!("{}?", ty(inner)),
        AstType::Result { ok, err, .. } => {
            format!("{} throws {}", ty(ok), ty(err))
        },
        AstType::Unit(_) => "()".into(),
        AstType::Never(_) => "Never".into(),
        AstType::Inferred(_) => "_".into(),
        AstType::Some {
            bounds, negative, ..
        } => {
            let b: Vec<_> = bounds.iter().map(ty).collect();
            match negative {
                Some(neg) => format!("some {} and not {}", b.join(" and "), ty(neg)),
                None => format!("some {}", b.join(" and ")),
            }
        },
        AstType::Ref {
            inner, mutating, ..
        } => {
            let kw = if *mutating { "&mutating " } else { "&" };
            format!("{kw}{}", ty(inner))
        },
    }
}

/// Resolve a conformance type to the protocol entity it refers to, by
/// matching the last path segment against known protocols. Stdlib has
/// unique protocol short-names so this works without full name resolution.
pub fn resolve_protocol(
    _world: &World,
    protocol_index: &std::collections::HashMap<String, Entity>,
    conformance: &AstType,
) -> Option<Entity> {
    let AstType::Named { segments, .. } = conformance else {
        return None;
    };
    let last = segments.last()?;
    protocol_index.get(&last.name).copied()
}
