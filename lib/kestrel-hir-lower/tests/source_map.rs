//! `BodySourceMap`: HIR ids ↔ the syntax they were lowered from.
//!
//! Each test lowers real source and checks the map against the text, so a
//! range that drifts off its identifier shows up as the wrong substring.

use std::sync::Arc;

use kestrel_ast_builder::{FileSyntax, Name, NodeKind, build_declarations};
use kestrel_hecs::{Entity, World};
use kestrel_hir::body::{HirExpr, HirName, HirPat, HirStmt};
use kestrel_hir::res::LocalId;
use kestrel_hir_lower::{LowerBodyWithSourceMap, LoweredBody};
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use rowan::TextSize;

struct Fixture {
    world: World,
    root: Entity,
    file: Entity,
    src: &'static str,
}

impl Fixture {
    fn new(src: &'static str) -> Self {
        let mut world = World::new();
        world.begin_revision();
        let root = world.spawn();
        world.set(root, NodeKind::Module);
        world.set(root, Name(Name::ROOT.into()));
        let file = world.spawn();
        let tokens: Vec<_> = kestrel_lexer::lex(src, file.index())
            .filter_map(|r| r.ok())
            .collect();
        let result = kestrel_parser::parse_source_file_from_source(
            src,
            tokens.iter().map(|t| (t.value.clone(), t.span.clone())),
        );
        build_declarations(&mut world, file, &result.tree(), root, None);
        Fixture {
            world,
            root,
            file,
            src,
        }
    }

    fn entity(&self, kind: NodeKind, name: &str) -> Entity {
        self.world
            .iter_component::<Name>()
            .find(|(e, n)| n.0 == name && self.world.get::<NodeKind>(*e) == Some(&kind))
            .map(|(e, _)| e)
            .unwrap_or_else(|| panic!("no {kind:?} `{name}`"))
    }

    fn lower(&self, entity: Entity) -> Arc<LoweredBody> {
        self.world
            .query_context()
            .query(LowerBodyWithSourceMap {
                entity,
                root: self.root,
            })
            .expect("body lowers")
    }

    fn func(&self, name: &str) -> Arc<LoweredBody> {
        self.lower(self.entity(NodeKind::Function, name))
    }

    fn tree(&self) -> SyntaxNode {
        self.world.get::<FileSyntax>(self.file).unwrap().root()
    }

    /// The offset of the `nth` (0-based) occurrence of `needle`, plus one so
    /// the cursor sits inside it.
    fn at(&self, needle: &str, nth: usize) -> TextSize {
        let start = self
            .src
            .match_indices(needle)
            .nth(nth)
            .unwrap_or_else(|| panic!("`{needle}` #{nth} not in source"))
            .0;
        TextSize::from(start as u32 + 1)
    }

    fn text(&self, range: rowan::TextRange) -> &str {
        &self.src[usize::from(range.start())..usize::from(range.end())]
    }
}

fn local_named(body: &LoweredBody, name: &str) -> LocalId {
    body.body
        .locals
        .iter()
        .find(|(_, l)| l.name == name)
        .map(|(id, _)| id)
        .unwrap_or_else(|| panic!("no local `{name}`"))
}

#[test]
fn let_binding_maps_to_its_identifier_not_the_statement() {
    let fx = Fixture::new("func f() -> Int { let count = 1; count }");
    let body = fx.func("f");
    let count = local_named(&body, "count");

    let source = body
        .source_map
        .local_source(count)
        .expect("declared in source");
    assert_eq!(fx.text(source.name), "count");
    assert_eq!(source.binding.kind(), SyntaxKind::BindingPattern);
    assert_eq!(
        body.source_map.local_declared_at(fx.at("count", 0)),
        Some(count)
    );

    // `Local::span` keeps its meaning (the whole statement): type inference
    // anchors "could not infer type" there (docs/fragility/F2/decisions.md).
    let span = &body.body.locals[count].span;
    assert_eq!(&fx.src[span.start..span.end], "let count = 1;");
}

#[test]
fn a_use_maps_to_its_local_expression() {
    let fx = Fixture::new("func f() -> Int { let count = 1; count }");
    let body = fx.func("f");
    let count = local_named(&body, "count");

    let expr = body
        .source_map
        .name_ref_at(fx.at("count", 1))
        .expect("use site");
    assert!(matches!(body.body.exprs[expr], HirExpr::Local(id, _) if id == count));
    // The declaration is not a use.
    assert_eq!(body.source_map.name_ref_at(fx.at("count", 0)), None);
}

#[test]
fn a_parameter_maps_to_its_name_in_the_signature() {
    let fx = Fixture::new("func f(label value: Int) -> Int { value }");
    let body = fx.func("f");
    let value = local_named(&body, "value");

    let source = body.source_map.local_source(value).expect("declared");
    assert_eq!(fx.text(source.name), "value");
    assert_eq!(
        body.source_map.local_declared_at(fx.at("value", 0)),
        Some(value)
    );
    // The label is not the binding.
    assert_eq!(body.source_map.local_declared_at(fx.at("label", 0)), None);
    let use_site = body.source_map.name_ref_at(fx.at("value", 1)).unwrap();
    assert!(matches!(body.body.exprs[use_site], HirExpr::Local(id, _) if id == value));
}

#[test]
fn synthesized_locals_have_no_source() {
    let fx = Fixture::new(
        "struct S { func m() -> Int { let (a, b) = (1, 2); a } }\n\
         func g(f: (Int) -> Int) -> Int { f(1) }\n\
         func h() -> Int { g({ it }) }",
    );
    let m = fx.func("m");
    for name in ["self", "$let_tmp"] {
        let id = local_named(&m, name);
        assert!(m.source_map.local_source(id).is_none(), "{name}");
    }
    // A destructuring binding is spelled in the source.
    let a = local_named(&m, "a");
    assert_eq!(fx.text(m.source_map.local_source(a).unwrap().name), "a");

    // An implicit `it` is declared nowhere, but its uses are uses.
    let h = fx.func("h");
    let it = local_named(&h, "it");
    assert!(h.source_map.local_source(it).is_none());
    let use_site = h.source_map.name_ref_at(fx.at("it }", 0)).unwrap();
    assert!(matches!(h.body.exprs[use_site], HirExpr::Local(id, _) if id == it));
}

#[test]
fn pattern_bindings_map_to_their_identifiers() {
    let fx = Fixture::new(
        "enum E { case A(Int) case B(Int) }\n\
         func f(e: E, xs: [Int]) -> Int {\n\
           match e { .A(x) => x, .B(var y) => y }\n\
         }\n\
         func g(xs: [Int]) -> Int { match xs { [first, ..rest] => first, whole @ _ => 0 } }\n\
         func h(c: (Int) -> Int) -> Int { c(1) }\n\
         func k() -> Int { h({ (p) in p }) }",
    );
    let f = fx.func("f");
    for name in ["x", "y"] {
        let id = local_named(&f, name);
        let source = f.source_map.local_source(id).expect(name);
        assert_eq!(fx.text(source.name), name);
    }
    let g = fx.func("g");
    for name in ["first", "rest", "whole"] {
        let id = local_named(&g, name);
        assert_eq!(
            fx.text(g.source_map.local_source(id).expect(name).name),
            name
        );
    }
    let k = fx.func("k");
    let p = local_named(&k, "p");
    assert_eq!(fx.text(k.source_map.local_source(p).unwrap().name), "p");
    assert_eq!(
        k.source_map
            .local_declared_at(fx.at("(p)", 0) + TextSize::from(1)),
        Some(p)
    );
}

#[test]
fn struct_shorthand_binding_is_not_recorded() {
    // `{ x }` binds a local named after the field: the token is the field's
    // name too, so renaming the local through it would rename the field.
    let fx = Fixture::new(
        "struct P { var x: Int }\n\
         func f(p: P) -> Int { match p { P { x } => x } }",
    );
    let f = fx.func("f");
    let x = local_named(&f, "x");
    assert!(f.source_map.local_source(x).is_none());
}

#[test]
fn subscript_parameters_are_not_recorded() {
    // Bound by the getter and again by each accessor: no single body owns
    // the declaration.
    let fx = Fixture::new("struct S { subscript(i: Int) -> Int { i } }");
    let subscript = fx
        .world
        .iter_component::<NodeKind>()
        .find(|(_, k)| **k == NodeKind::Subscript)
        .map(|(e, _)| e)
        .unwrap();
    let sub = fx.lower(subscript);
    let i = local_named(&sub, "i");
    assert!(sub.source_map.local_source(i).is_none());
}

#[test]
fn expressions_patterns_and_statements_round_trip() {
    let fx = Fixture::new("func f() -> Int { let n = (40 + 2); match n { 42 => n, _ => 0 } }");
    let body = fx.func("f");
    let tree = fx.tree();

    // Every recorded expression points at a node that maps back to it, or
    // (for a grouping) at the node it was lowered from.
    for (id, _) in body.body.exprs.iter() {
        if let Some(ptr) = body.source_map.expr_syntax(id) {
            let node = ptr.to_node(&tree);
            assert_eq!(body.source_map.node_expr(&node), Some(id), "{node:?}");
        }
    }

    let lit = body.source_map.expr_at(&tree, fx.at("40", 0)).unwrap();
    assert!(matches!(body.body.exprs[lit], HirExpr::Literal { .. }));
    // `(40 + 2)` is transparent: the grouping maps to the sum's expression
    // (an error here — there is no stdlib to supply `+`), which points back
    // at the binary node.
    let sum = body.source_map.expr_at(&tree, fx.at("+", 0)).unwrap();
    let sum_node = body.source_map.expr_syntax(sum).unwrap();
    assert_eq!(sum_node.kind(), SyntaxKind::ExprBinary);
    let grouping = sum_node.to_node(&tree).parent().unwrap().parent().unwrap();
    assert_eq!(grouping.kind(), SyntaxKind::ExprGrouping);
    assert_eq!(body.source_map.node_expr(&grouping), Some(sum));

    let pat = body.source_map.pat_at(&tree, fx.at("42", 0)).unwrap();
    assert!(matches!(body.body.pats[pat], HirPat::Literal { .. }));

    let let_node = tree
        .descendants()
        .find(|n| n.kind() == SyntaxKind::VariableDeclaration)
        .unwrap();
    let stmt = body.source_map.node_stmt(&let_node).expect("let statement");
    assert!(matches!(body.body.stmts[stmt], HirStmt::Let { .. }));
}

#[test]
fn member_names_map_to_their_own_access() {
    // `make().inner.value`: a computed base, then two member accesses. The
    // cursor on each name finds that name's `Field`, not the outermost one.
    let fx = Fixture::new(
        "struct In { var value: Int }\n\
         struct Out { var inner: In }\n\
         func make() -> Out { Out(inner: In(value: 1)) }\n\
         func f() -> Int { make().inner.value }",
    );
    let body = fx.func("f");
    for name in ["inner", "value"] {
        let at = fx.src.rfind(name).unwrap() as u32 + 1;
        let id = body.source_map.name_ref_at(TextSize::from(at)).unwrap();
        let HirExpr::Field { name: field, .. } = &body.body.exprs[id] else {
            panic!("{name}: not a field");
        };
        assert_eq!(field, &HirName::name(name));
    }
}
