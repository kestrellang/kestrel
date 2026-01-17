//! Tests for cycle detection validation passes.
//!
//! These tests verify that the compiler correctly detects:
//! - Struct containment cycles (infinite-size types)
//! - Generic constraint cycles
//! - Protocol inheritance cycles

use kestrel_test_suite::*;

mod struct_cycles {
    use super::*;

    #[test]
    fn direct_self_reference_error() {
        // Struct containing itself directly
        Test::new(
            r#"
module Main

struct Node {
    let next: Node
}
"#,
        )
        .expect(HasError("cannot contain itself"))
        .expect(HasError("Node"));
    }

    #[test]
    fn two_struct_cycle_error() {
        // Two structs containing each other
        Test::new(
            r#"
module Main

struct A {
    let b: B
}

struct B {
    let a: A
}
"#,
        )
        .expect(HasError("circular struct containment"));
    }

    #[test]
    fn three_struct_cycle_error() {
        // Three structs in a cycle: A -> B -> C -> A
        Test::new(
            r#"
module Main

struct A {
    let b: B
}

struct B {
    let c: C
}

struct C {
    let a: A
}
"#,
        )
        .expect(HasError("circular struct containment"));
    }

    #[test]
    fn tuple_with_cycle_error() {
        // Tuple containing struct that references parent
        Test::new(
            r#"
module Main

struct A {
    let pair: (Int, B)
}

struct B {
    let a: A
}
"#,
        )
        .expect(HasError("circular struct containment"));
    }

    #[test]
    fn array_breaks_cycle() {
        // Arrays use indirection, so this should be allowed
        Test::new(
            r#"
module Main

struct Node {
    let children: [Node]
}
"#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Node")
                .is(SymbolKind::Struct)
                .has(Behavior::FieldCount(1)),
        );
    }

    #[test]
    fn self_reference_in_array_ok() {
        // Self-reference through array is ok (indirect)
        Test::new(
            r#"
module Main

struct TreeNode {
    let value: Int
    let children: [TreeNode]
}
"#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("TreeNode")
                .is(SymbolKind::Struct)
                .has(Behavior::FieldCount(2)),
        );
    }

    #[test]
    fn no_cycle_acyclic_references() {
        // Structs referencing each other but no cycle (Point <- Line, Point <- Triangle)
        Test::new(
            r#"
module Main

struct Point {
    let x: Int
    let y: Int
}

struct Line {
    let start: Point
    let end: Point
}

struct Triangle {
    let a: Point
    let b: Point
    let c: Point
}
"#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Point")
                .is(SymbolKind::Struct)
                .has(Behavior::FieldCount(2)),
        )
        .expect(
            Symbol::new("Line")
                .is(SymbolKind::Struct)
                .has(Behavior::FieldCount(2)),
        )
        .expect(
            Symbol::new("Triangle")
                .is(SymbolKind::Struct)
                .has(Behavior::FieldCount(3)),
        );
    }

    #[test]
    fn nested_structs_valid_chain() {
        // Deep nesting with no cycle: Inner <- Middle <- Outer
        Test::new(
            r#"
module Main

struct Inner {
    let value: Int
}

struct Middle {
    let inner: Inner
}

struct Outer {
    let middle: Middle
}
"#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Inner")
                .is(SymbolKind::Struct)
                .has(Behavior::FieldCount(1)),
        )
        .expect(
            Symbol::new("Middle")
                .is(SymbolKind::Struct)
                .has(Behavior::FieldCount(1)),
        )
        .expect(
            Symbol::new("Outer")
                .is(SymbolKind::Struct)
                .has(Behavior::FieldCount(1)),
        );
    }
}

mod constraint_cycles {
    use super::*;

    #[test]
    fn mutual_constraint_reference_rejected() {
        // T's bound references U, U's bound references T through protocol generic.
        // The compiler rejects this as a circular generic constraint.
        Test::new(
            r#"
module Main

protocol Container[T] {
    func get() -> T
}

func swap[T, U](a: T, b: U) -> () where T: Container[U], U: Container[T] {
    ()
}
"#,
        )
        .expect(HasError("circular generic constraint"));
    }

    #[test]
    fn no_cycle_independent_constraints() {
        // Independent constraints, no cycle
        Test::new(
            r#"
module Main

protocol Printable {
    func print() -> String
}

protocol Comparable {
    func compare() -> Int
}

func process[T, U](a: T, b: U) -> () where T: Printable, U: Comparable {
    ()
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Printable").is(SymbolKind::Protocol))
        .expect(Symbol::new("Comparable").is(SymbolKind::Protocol));
    }

    #[test]
    fn single_constraint_generic_struct() {
        // Single constraint with generic struct, no cycle possible
        Test::new(
            r#"
module Main

protocol Hashable {
    func hash() -> Int
}

struct Set[T] where T: Hashable {
    let items: [T]
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Hashable").is(SymbolKind::Protocol));
    }
}

mod protocol_inheritance_cycles {
    use super::*;

    #[test]
    fn direct_protocol_self_inheritance() {
        // Protocol conforming to itself
        Test::new(
            r#"
module Main

protocol Recursive: Recursive {
    func method() -> Int
}
"#,
        )
        .expect(HasError("circular"));
    }

    #[test]
    fn two_protocol_cycle() {
        // Two protocols inheriting from each other
        Test::new(
            r#"
module Main

protocol A: B {
    func methodA() -> Int
}

protocol B: A {
    func methodB() -> Int
}
"#,
        )
        .expect(HasError("circular"));
    }

    #[test]
    fn three_protocol_cycle() {
        // Three protocols in inheritance cycle
        Test::new(
            r#"
module Main

protocol A: B {
    func a() -> Int
}

protocol B: C {
    func b() -> Int
}

protocol C: A {
    func c() -> Int
}
"#,
        )
        .expect(HasError("circular"));
    }

    #[test]
    fn linear_protocol_inheritance_ok() {
        // Linear chain, no cycle: Base <- Middle <- Derived
        Test::new(
            r#"
module Main

protocol Base {
    func base() -> Int
}

protocol Middle: Base {
    func middle() -> Int
}

protocol Derived: Middle {
    func derived() -> Int
}
"#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Base")
                .is(SymbolKind::Protocol)
                .has(Behavior::ConformanceCount(0)),
        )
        .expect(
            Symbol::new("Middle")
                .is(SymbolKind::Protocol)
                .has(Behavior::ConformanceCount(1)),
        )
        .expect(
            Symbol::new("Derived")
                .is(SymbolKind::Protocol)
                .has(Behavior::ConformanceCount(1)),
        );
    }

    #[test]
    fn diamond_inheritance_ok() {
        // Diamond pattern (A <- B, A <- C) is not a cycle
        Test::new(
            r#"
module Main

protocol A {
    func a() -> lang.i64
}

protocol B: A {
    func b() -> lang.i64
}

protocol C: A {
    func c() -> lang.i64
}
"#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("A")
                .is(SymbolKind::Protocol)
                .has(Behavior::ConformanceCount(0)),
        )
        .expect(
            Symbol::new("B")
                .is(SymbolKind::Protocol)
                .has(Behavior::ConformanceCount(1)),
        )
        .expect(
            Symbol::new("C")
                .is(SymbolKind::Protocol)
                .has(Behavior::ConformanceCount(1)),
        );
    }
}
