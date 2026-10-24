//! Parsing for existentials (docs/plans/existentials): `any` types, `as?`,
//! the `is` operator and `is` patterns.

mod common;

use common::{first, parse, parse_ok};
use kestrel_syntax_tree::{SyntaxKind as K, SyntaxNode};

/// Kinds of the non-trivia children of `node`, nodes and tokens alike.
fn child_kinds(node: &SyntaxNode) -> Vec<K> {
    node.children_with_tokens()
        .map(|c| c.kind())
        .filter(|k| !k.is_trivia())
        .collect()
}

fn count(tree: &SyntaxNode, kind: K) -> usize {
    tree.descendants().filter(|n| n.kind() == kind).count()
}

#[test]
fn any_type_forms() {
    let tree = parse_ok(
        "func f(a: any Shape, b: any[UniqueBox] Shape and Printable, \
         c: any Iterator[Item = Int64], d: Array[any Shape]) {}\n",
    );
    assert_eq!(count(&tree, K::TyAny), 4);
    let binding = first(&tree, K::TypeArgBinding);
    assert_eq!(child_kinds(&binding), vec![K::Identifier, K::Equals, K::Ty]);
}

#[test]
fn any_stays_an_identifier_outside_type_position() {
    let tree = parse_ok("func f() { let b = xs.any(where: { it > 1 }); let any = 1; }\n");
    assert_eq!(count(&tree, K::TyAny), 0);
}

#[test]
fn as_question_binds_tighter_than_coalesce_and_arithmetic() {
    // `x as? T ?? d` is `(x as? T) ?? d`: the cast target takes no `?` suffix.
    let tree = parse_ok("func f() { let a = x as? Circle ?? d; }\n");
    let bin = first(&tree, K::ExprBinary);
    let lhs = bin.first_child().unwrap().first_child().unwrap();
    assert_eq!(lhs.kind(), K::ExprCast);

    // `a + b as? T` is `a + (b as? T)`.
    let tree = parse_ok("func f() { let a = n + m as? Circle; }\n");
    let bin = first(&tree, K::ExprBinary);
    let rhs = bin.children().nth(1).unwrap().first_child().unwrap();
    assert_eq!(rhs.kind(), K::ExprCast);
}

#[test]
fn is_operator_is_a_comparison() {
    let tree = parse_ok("func f() { let a = x is Circle and y is not Square; }\n");
    assert_eq!(count(&tree, K::ExprTypeTest), 2);
    let and = first(&tree, K::ExprBinary);
    assert!(child_kinds(&and).contains(&K::And));
    let not = first(&tree, K::ExprTypeTest)
        .descendants_with_tokens()
        .any(|e| e.kind() == K::Not);
    assert!(!not, "the first test has no `not`");
    let second = tree
        .descendants()
        .filter(|n| n.kind() == K::ExprTypeTest)
        .nth(1)
        .unwrap();
    assert!(child_kinds(&second).contains(&K::Not));
}

#[test]
fn consuming_cast_in_a_match_head() {
    let tree =
        parse_ok("func f() { match consuming a as? Circle { .Ok(c) => 1, .Err(_) => 2 } }\n");
    let cast = first(&tree, K::ExprCast);
    assert_eq!(child_kinds(&cast)[0], K::Consuming);
    assert!(
        !parse("func f() { let a = consuming x; }\n")
            .errors
            .is_empty(),
        "`consuming` needs `as?`"
    );
}

#[test]
fn is_pattern_forms() {
    let tree = parse_ok(
        "func f() { match s { is Circle => 1, c is Square => 2, \
         is Rect { w: 0, .. } => 3, r is Rect { w, .. } => 4, var v is Circle => 5, \
         is Circle or is Square => 6, x @ is Circle => 7, _ => 0 } }\n",
    );
    assert_eq!(count(&tree, K::IsPattern), 8);
    assert_eq!(count(&tree, K::StructPatternField), 2);
}

#[test]
fn is_pattern_in_let_conditions() {
    parse_ok(
        "func f() { if let c is Circle = s { } guard let q is Square = s else { return } \
         while let q is Square = xs(i) { } }\n",
    );
}

/// Every existential test program parses cleanly, so the conformance test
/// (which skips files with parse errors) actually covers them.
#[test]
fn existential_testdata_parses() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../kestrel-test-suite/testdata/types/existential");
    let mut checked = 0;
    let mut failures = Vec::new();
    for entry in std::fs::read_dir(&dir).expect("testdata dir").flatten() {
        let path = entry.path();
        let source = std::fs::read_to_string(&path).unwrap();
        let errors = parse(&source).errors;
        if let Some(e) = errors.first() {
            failures.push(format!("{}: {}", path.display(), e.message));
        }
        checked += 1;
    }
    assert!(checked >= 30, "found only {checked} files");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn assoc_binding_outside_any_is_e810() {
    let errors = parse("func f(x: Array[Element = Int64]) {}\n").errors;
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].code, Some("E810"));
    let errors = parse("func f(x: some Iterator[Item = Int64]) {}\n").errors;
    assert_eq!(errors.len(), 1, "{errors:?}");
}
