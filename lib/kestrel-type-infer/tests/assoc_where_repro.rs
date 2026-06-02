//! Repro harness for the two stdlib associated-type-equality inference errors
//! that block `kestrel build` (adapters.ks `ChainIterator.next`, array.ks
//! `Array.flatten`). Self-contained: defines minimal protocols inline rather
//! than loading stdlib.

use kestrel_ast_builder::{Name, NodeKind, build_declarations, seed_lang_module};
use kestrel_hecs::{Entity, World};
use kestrel_type_infer::InferBody;
use kestrel_type_infer::result::TypedBody;

fn build_from_source(source: &str) -> (World, Entity) {
    let mut world = World::new();
    world.begin_revision();
    let root = world.spawn();
    world.set(root, NodeKind::Module);
    world.set(root, Name("<root>".to_string()));
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

fn child(ctx: &kestrel_hecs::QueryContext<'_>, parent: Entity, kind: NodeKind, name: &str) -> Entity {
    ctx.children_of(parent)
        .iter()
        .find(|&&e| {
            ctx.get::<NodeKind>(e) == Some(&kind) && ctx.get::<Name>(e).is_some_and(|n| n.0 == name)
        })
        .copied()
        .unwrap_or_else(|| panic!("child {:?} {:?} not found under {:?}", kind, name, parent))
}

/// Find a method by name under any child of `module` of `container_kind`
/// (used for extensions, which have no Name).
fn method_under_kind(
    ctx: &kestrel_hecs::QueryContext<'_>,
    module: Entity,
    container_kind: NodeKind,
    method: &str,
) -> Entity {
    for &c in ctx.children_of(module) {
        if ctx.get::<NodeKind>(c) != Some(&container_kind) {
            continue;
        }
        if let Some(&m) = ctx.children_of(c).iter().find(|&&e| {
            ctx.get::<NodeKind>(e) == Some(&NodeKind::Function)
                && ctx.get::<Name>(e).is_some_and(|n| n.0 == method)
        }) {
            return m;
        }
    }
    panic!("method {method} not found under any {container_kind:?}");
}

fn infer(ctx: &kestrel_hecs::QueryContext<'_>, root: Entity, m: Entity) -> TypedBody {
    ctx.query(InferBody { entity: m, root }).expect("InferBody None")
}

// NOTE: bug 1 — struct-level associated-type-equality where clauses
// (`struct ChainIterator[A,B] ... where B.Item = A.Item`) being invisible in
// the struct's own method bodies — is fixed in `create_param_types` (struct/
// enum methods now run `emit_container_where_clauses`). It is NOT unit-tested
// here: this minimal `build_from_source` harness doesn't wire enough
// name-resolution for `ResolveTypePath` to resolve a module-level protocol
// from a *method* context (it works from a free-function context), so the
// where-clause bounds never gather. The real compiler resolves them fine — the
// regression guard for bug 1 is stdlib `iter/adapters.ks` (`ChainIterator`)
// type-checking, exercised by every `kestrel build`.

/// Bug 2 — protocol associated-type-equality (`Iterable.TargetIterator.Item =
/// Item`) not propagated for an extension bound `T: Iterable`. `flatten()`
/// drains `T.TargetIterator.Item` into an `Array[T.Item]`.
#[test]
fn extension_bound_propagates_protocol_assoc_equality() {
    let source = r#"
module TestMod
protocol Seq {
    type Item
    func next() -> Item
}
protocol Collection {
    type Item
    type Target: Seq where Target.Item = Item
    func iter() -> Target
}
struct Wrap[T] where T: Collection {
    var inner: T
}
extend Wrap[T] where T: Collection {
    func first() -> T.Item {
        var it = self.inner.iter();
        it.next()
    }
}
"#;
    let (world, root) = build_from_source(source);
    let ctx = world.query_context();
    let module = child(&ctx, root, NodeKind::Module, "TestMod");
    let first = method_under_kind(&ctx, module, NodeKind::Extension, "first");
    let typed = infer(&ctx, root, first);
    assert!(
        typed.errors.is_empty(),
        "protocol assoc equality `Target.Item = Item` not propagated for `T: Collection`: {:#?}",
        typed.errors
    );
}
