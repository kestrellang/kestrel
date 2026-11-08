//! Parsing for `unsafe`: the declaration modifier, the `unsafe expr` prefix,
//! the `unsafe { … }` block and `unsafe P` conformances.

mod common;

use common::{first, parse_ok};
use kestrel_syntax_tree::{SyntaxKind as K, SyntaxNode};

fn count(tree: &SyntaxNode, kind: K) -> usize {
    tree.descendants().filter(|n| n.kind() == kind).count()
}

#[test]
fn unsafe_modifier_on_declarations() {
    let tree = parse_ok(
        "public unsafe protocol P {}\n\
         struct S {\n\
             public unsafe init(x: Int64) {}\n\
             static unsafe func make() -> S { S(x: 1) }\n\
             unsafe mutating func poke() {}\n\
             unsafe subscript(i: Int64) -> Int64 { 0 }\n\
             unsafe var raw: Int64 { 0 }\n\
         }\n\
         unsafe func top() {}\n",
    );
    assert_eq!(count(&tree, K::UnsafeModifier), 7);
}

#[test]
fn unsafe_prefix_binds_like_try() {
    // `unsafe p.read() + 1` is `(unsafe p.read()) + 1`.
    let tree = parse_ok("func f() { let a = unsafe p.read() + 1; }\n");
    let bin = first(&tree, K::ExprBinary);
    let lhs = bin.first_child().unwrap().first_child().unwrap();
    assert_eq!(lhs.kind(), K::ExprUnsafe);
}

#[test]
fn unsafe_brace_is_a_block_not_a_closure() {
    let tree = parse_ok(
        "func f() -> Int64 {\n\
             unsafe { p.write(1); }\n\
             let n = unsafe { p.read() };\n\
             n\n\
         }\n",
    );
    assert_eq!(count(&tree, K::ExprUnsafeBlock), 2);
    assert_eq!(count(&tree, K::ExprClosure), 0);
}

#[test]
fn unsafe_prefix_on_a_parenthesised_closure_stays_a_closure() {
    let tree = parse_ok("func f() { let g = unsafe ({ it }); }\n");
    assert_eq!(count(&tree, K::ExprUnsafe), 1);
    assert_eq!(count(&tree, K::ExprClosure), 1);
}

#[test]
fn unsafe_conformance() {
    let tree = parse_ok("extend Box: unsafe ArrayMatchable, Equatable {}\n");
    assert_eq!(count(&tree, K::UnsafeConformance), 1);
}
