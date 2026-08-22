//! `WhereSubject` shape tests — what `WhereClausesOf` records as the *subject*
//! of a bound, at each spelling and each depth.
//!
//! These assert on representation, not on inference: a projection subject is
//! still skipped by every consumer (see `docs/fragility/G14-G17/decisions.md`),
//! so the only way to see whether the receiver survived resolution is to read
//! the clause back. Same lex → parse → `build_declarations` harness as
//! `assoc_where_repro.rs`; a hand-built `World` cannot resolve `C.Iter.Item`.

use kestrel_ast_builder::{Name, NodeKind, build_declarations, seed_lang_module};
use kestrel_hecs::{Entity, QueryContext, World};
use kestrel_type_infer::resolve::{WhereClause, WhereSubject};
use kestrel_type_infer::where_clauses::WhereClausesOf;

fn build_from_source(source: &str) -> (World, Entity) {
    let mut world = World::new();
    world.begin_revision();
    let root = world.spawn();
    world.set(root, NodeKind::Module);
    world.set(root, Name(Name::ROOT.to_string()));
    seed_lang_module(&mut world, root);

    let file_entity = world.spawn();
    let tokens: Vec<_> = kestrel_lexer::lex(source, file_entity.index())
        .filter_map(|r| r.ok())
        .collect();
    let token_iter = tokens.iter().map(|t| (t.value.clone(), t.span.clone()));
    let result = kestrel_parser::parse_source_file_from_source(source, token_iter);
    build_declarations(&mut world, file_entity, &result.tree, root, None);
    (world, root)
}

fn child(ctx: &QueryContext<'_>, parent: Entity, kind: NodeKind, name: &str) -> Entity {
    ctx.children_of(parent)
        .iter()
        .find(|&&e| {
            ctx.get::<NodeKind>(e) == Some(&kind) && ctx.get::<Name>(e).is_some_and(|n| n.0 == name)
        })
        .copied()
        .unwrap_or_else(|| panic!("child {kind:?} {name:?} not found under {parent:?}"))
}

/// The only `Extension` under `parent` (these sources declare exactly one).
fn sole_extension(ctx: &QueryContext<'_>, parent: Entity) -> Entity {
    ctx.children_of(parent)
        .iter()
        .find(|&&e| ctx.get::<NodeKind>(e) == Some(&NodeKind::Extension))
        .copied()
        .expect("no extension under module")
}

/// Subjects of every `Bound` clause on `entity` whose protocol is `protocol`.
fn subjects_for_protocol(
    ctx: &QueryContext<'_>,
    entity: Entity,
    root: Entity,
    protocol: Entity,
) -> Vec<WhereSubject> {
    ctx.query(WhereClausesOf { entity, root })
        .iter()
        .filter_map(|wc| match wc {
            WhereClause::Bound {
                subject,
                protocol: p,
                ..
            } if *p == protocol => Some(subject.clone()),
            _ => None,
        })
        .collect()
}

/// `protocol Equatable`/`Iterator`/`Container` + a free function carrying the
/// where clause under test. Free-function context is deliberate: the harness
/// under-wires name resolution from a *method* context (see the note in
/// `assoc_where_repro.rs`).
fn container_source(where_clause: &str) -> String {
    format!(
        r#"
module TestMod
protocol Equatable {{ }}
protocol Iterator {{
    type Item;
}}
protocol Container {{
    type Iter: Iterator;
}}
func findIn[C](c: C) where {where_clause} {{ }}
"#
    )
}

#[test]
fn bare_param_subject_is_param() {
    let (world, root) = build_from_source(&container_source("C: Container"));
    let ctx = world.query_context();
    let module = child(&ctx, root, NodeKind::Module, "TestMod");
    let container = child(&ctx, module, NodeKind::Protocol, "Container");
    let find_in = child(&ctx, module, NodeKind::Function, "findIn");
    let c = child(&ctx, find_in, NodeKind::TypeParameter, "C");

    assert_eq!(
        subjects_for_protocol(&ctx, find_in, root, container),
        vec![WhereSubject::Param(c)]
    );
}

#[test]
fn depth_two_projection_keeps_its_base() {
    let (world, root) = build_from_source(&container_source("C: Container, C.Iter: Iterator"));
    let ctx = world.query_context();
    let module = child(&ctx, root, NodeKind::Module, "TestMod");
    let iterator = child(&ctx, module, NodeKind::Protocol, "Iterator");
    let container = child(&ctx, module, NodeKind::Protocol, "Container");
    let iter = child(&ctx, container, NodeKind::TypeAlias, "Iter");
    let find_in = child(&ctx, module, NodeKind::Function, "findIn");
    let c = child(&ctx, find_in, NodeKind::TypeParameter, "C");

    assert_eq!(
        subjects_for_protocol(&ctx, find_in, root, iterator),
        vec![WhereSubject::Projection {
            base: Box::new(WhereSubject::Param(c)),
            assoc: iter,
        }]
    );
}

/// The capability added by D7 commit 2: `C.Iter.Item` no longer collapses to
/// a bare `Bound { subject: Param(Item) }` with its receiver thrown away.
/// Every consumer still *skips* this subject — the point is that the clause
/// now records enough for stage 3a to stop skipping it.
#[test]
fn nested_projection_nests_to_depth_three() {
    let (world, root) =
        build_from_source(&container_source("C: Container, C.Iter.Item: Equatable"));
    let ctx = world.query_context();
    let module = child(&ctx, root, NodeKind::Module, "TestMod");
    let equatable = child(&ctx, module, NodeKind::Protocol, "Equatable");
    let iterator = child(&ctx, module, NodeKind::Protocol, "Iterator");
    let item = child(&ctx, iterator, NodeKind::TypeAlias, "Item");
    let container = child(&ctx, module, NodeKind::Protocol, "Container");
    let iter = child(&ctx, container, NodeKind::TypeAlias, "Iter");
    let find_in = child(&ctx, module, NodeKind::Function, "findIn");
    let c = child(&ctx, find_in, NodeKind::TypeParameter, "C");

    assert_eq!(
        subjects_for_protocol(&ctx, find_in, root, equatable),
        vec![WhereSubject::Projection {
            base: Box::new(WhereSubject::Projection {
                base: Box::new(WhereSubject::Param(c)),
                assoc: iter,
            }),
            assoc: item,
        }]
    );
}

/// D8: `Self` subjects keep collapsing to the enclosing entity until stage 3a.
/// `WhereSubject::SelfType` exists but is never constructed — when someone
/// flips the producer, this test is what tells them what they changed.
#[test]
fn self_subject_still_collapses() {
    let source = r#"
module TestMod
protocol Comparable {
    func compare(other: Self)
}
protocol Equatable {
    func isEqual(to other: Self)
}
protocol Iterator {
    type Item
    func next()
}
extend Iterator where Self: Comparable, Self.Item: Equatable {
    func mixedHelper() { }
}
"#;
    let (world, root) = build_from_source(source);
    let ctx = world.query_context();
    let module = child(&ctx, root, NodeKind::Module, "TestMod");
    let comparable = child(&ctx, module, NodeKind::Protocol, "Comparable");
    let equatable = child(&ctx, module, NodeKind::Protocol, "Equatable");
    let iterator = child(&ctx, module, NodeKind::Protocol, "Iterator");
    let item = child(&ctx, iterator, NodeKind::TypeAlias, "Item");
    let extension = sole_extension(&ctx, module);

    // `Self: Comparable` → the extension target, NOT `WhereSubject::SelfType`.
    assert_eq!(
        subjects_for_protocol(&ctx, extension, root, comparable),
        vec![WhereSubject::Param(iterator)]
    );
    // `Self.Item: Equatable` → collapsed to the assoc entity alone, receiver
    // dropped. A `Self`-rooted chain does not gain depth in this commit.
    assert_eq!(
        subjects_for_protocol(&ctx, extension, root, equatable),
        vec![WhereSubject::Param(item)]
    );
}

/// The implicit `T: Copyable` injection (`inject_implicit_copyable_bounds`)
/// builds its subject directly rather than through `resolve_bound_subject`,
/// so it needs its own guard.
#[test]
fn implicit_copyable_bound_uses_param_subject() {
    let source = r#"
module TestMod
// `@builtin` because the harness has no stdlib: `ResolveBuiltin` is
// name-first, but the name lookup runs in root scope and does not see into
// `TestMod`. The attribute index does.
@builtin(.Copyable)
protocol Copyable { }
func identity[T](x: T) { }
"#;
    let (world, root) = build_from_source(source);
    let ctx = world.query_context();
    let module = child(&ctx, root, NodeKind::Module, "TestMod");
    let copyable = child(&ctx, module, NodeKind::Protocol, "Copyable");
    let identity = child(&ctx, module, NodeKind::Function, "identity");
    let t = child(&ctx, identity, NodeKind::TypeParameter, "T");

    assert_eq!(
        subjects_for_protocol(&ctx, identity, root, copyable),
        vec![WhereSubject::Param(t)],
        "implicit Copyable bound must carry a bare-param subject"
    );
}
