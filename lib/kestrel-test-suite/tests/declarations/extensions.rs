//! Tests for extension declarations
//!
//! Extensions allow adding methods and protocol conformances to existing types.
//!
//! Syntax:
//! ```kestrel
//! extend Type: Protocol {
//!     func method() { ... }
//! }
//! ```

use kestrel_test_suite::*;

mod basic {
    use super::*;

    #[test]
    fn empty_extension() {
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            extend Point { }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Point").is(SymbolKind::Struct));
    }

    #[test]
    fn extension_with_method() {
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            extend Point {
                func describe() -> String { return "a point"; }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Point").is(SymbolKind::Struct));
    }

    #[test]
    fn extension_method_accessible() {
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            extend Point {
                func sum() -> Int { return self.x + self.y; }
            }
            func test() -> Int {
                let p = Point(x: 3, y: 4);
                return p.sum();
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn extension_with_multiple_methods() {
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            extend Point {
                func sum() -> Int { return self.x + self.y; }
                func product() -> Int { return self.x * self.y; }
            }
            func test() -> Int {
                let p = Point(x: 3, y: 4);
                return p.sum() + p.product();
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn multiple_extensions_same_type() {
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            extend Point {
                func sum() -> Int { return self.x + self.y; }
            }
            extend Point {
                func product() -> Int { return self.x * self.y; }
            }
            func test() -> Int {
                let p = Point(x: 3, y: 4);
                return p.sum() + p.product();
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod conformance {
    use super::*;

    #[test]
    fn extension_adds_conformance() {
        Test::new(
            r#"module Test
            protocol Describable { func describe() -> String }
            struct Point { var x: Int; var y: Int }
            extend Point: Describable {
                func describe() -> String { return "a point"; }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn extension_satisfies_protocol_method() {
        Test::new(
            r#"module Test
            protocol Hashable { func hash() -> Int }
            struct Point { var x: Int; var y: Int }
            extend Point: Hashable {
                func hash() -> Int { return self.x + self.y; }
            }
            func getHash[T](value: T) -> Int where T: Hashable { return value.hash(); }
            func test() -> Int {
                let p = Point(x: 3, y: 4);
                return getHash(p);
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn extension_missing_protocol_method() {
        Test::new(
            r#"module Test
            protocol Describable { func describe() -> String }
            struct Point { var x: Int; var y: Int }
            extend Point: Describable { }
        "#,
        )
        .expect(HasError("does not implement method 'describe'"));
    }

    #[test]
    fn extension_multiple_conformances() {
        Test::new(
            r#"module Test
            protocol Hashable { func hash() -> Int }
            protocol Describable { func describe() -> String }
            struct Point { var x: Int; var y: Int }
            extend Point: Hashable, Describable {
                func hash() -> Int { return self.x + self.y; }
                func describe() -> String { return "point"; }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn extension_inherits_protocol_methods() {
        Test::new(
            r#"module Test
            protocol Base { func base() -> Int }
            protocol Child: Base { func child() -> Int }
            struct Point { var x: Int; var y: Int }
            extend Point: Child {
                func base() -> Int { return self.x; }
                func child() -> Int { return self.y; }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn separate_extensions_satisfy_conformance() {
        Test::new(
            r#"module Test
            protocol Hashable { func hash() -> Int }
            struct Point { var x: Int; var y: Int }
            extend Point {
                func hash() -> Int { return self.x + self.y; }
            }
            extend Point: Hashable { }
        "#,
        )
        .expect(Compiles);
    }
}

mod generics {
    use super::*;

    #[test]
    fn extension_generic_struct() {
        Test::new(
            r#"module Test
            struct Box[T] { var value: T }
            extend Box[T] {
                func get() -> T { return self.value; }
            }
            func test() -> Int {
                let b = Box[Int](value: 42);
                return b.get();
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn extension_specialized_concrete() {
        Test::new(
            r#"module Test
            struct Box[T] { var value: T }
            extend Box[Int] {
                func doubled() -> Int { return self.value * 2; }
            }
            func test() -> Int {
                let b = Box[Int](value: 21);
                return b.doubled();
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn extension_specialized_not_on_wrong_type() {
        Test::new(
            r#"module Test
            struct Box[T] { var value: T }
            extend Box[Int] {
                func doubled() -> Int { return self.value * 2; }
            }
            func test() -> Int {
                let b = Box[String](value: "hello");
                return b.doubled();
            }
        "#,
        )
        .expect(HasError("doubled"));
    }

    #[test]
    fn extension_mixed_type_params() {
        Test::new(
            r#"module Test
            struct Pair[T, U] { var first: T; var second: U }
            extend Pair[T, Int] {
                func getSecond() -> Int { return self.second; }
            }
            func test() -> Int {
                let p = Pair[String, Int](first: "hello", second: 42);
                return p.getSecond();
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn extension_mixed_not_applicable() {
        Test::new(
            r#"module Test
            struct Pair[T, U] { var first: T; var second: U }
            extend Pair[T, Int] {
                func getSecond() -> Int { return self.second; }
            }
            func test() -> String {
                let p = Pair[String, String](first: "hello", second: "world");
                return p.getSecond();
            }
        "#,
        )
        .expect(HasError("getSecond"));
    }

    #[test]
    fn extension_uses_type_param_in_method() {
        Test::new(
            r#"module Test
            struct Box[T] { var value: T }
            extend Box[T] {
                func getValue() -> T { return self.value; }
                mutating func setValue(newValue: T) { self.value = newValue; }
            }
            func test() -> Int {
                var b = Box[Int](value: 10);
                b.setValue(20);
                return b.getValue();
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn extension_two_type_params_generic() {
        Test::new(
            r#"module Test
            struct Pair[T, U] { var first: T; var second: U }
            extend Pair[T, U] {
                func getFirst() -> T { return self.first; }
                func getSecond() -> U { return self.second; }
            }
            func test() -> Int {
                let p = Pair[String, Int](first: "hello", second: 42);
                return p.getSecond();
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod specialization {
    use super::*;

    #[test]
    fn specialized_extension_wins() {
        Test::new(
            r#"module Test
            struct Box[T] { var value: T }
            extend Box[T] {
                func describe() -> String { return "generic box"; }
            }
            extend Box[Int] {
                func describe() -> String { return "int box"; }
            }
            func testGeneric() -> String {
                let b = Box[String](value: "hello");
                return b.describe();
            }
            func testSpecialized() -> String {
                let b = Box[Int](value: 42);
                return b.describe();
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn more_specialized_wins() {
        Test::new(
            r#"module Test
            struct Pair[T, U] { var first: T; var second: U }
            extend Pair[T, U] {
                func describe() -> String { return "generic pair"; }
            }
            extend Pair[T, Int] {
                func describe() -> String { return "half specialized"; }
            }
            extend Pair[Int, Int] {
                func describe() -> String { return "fully specialized"; }
            }
            func test1() -> String {
                let p = Pair[String, String](first: "a", second: "b");
                return p.describe();
            }
            func test2() -> String {
                let p = Pair[String, Int](first: "a", second: 1);
                return p.describe();
            }
            func test3() -> String {
                let p = Pair[Int, Int](first: 1, second: 2);
                return p.describe();
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod where_clause {
    use super::*;

    #[test]
    fn extension_with_where_clause() {
        Test::new(
            r#"module Test
            protocol Equatable { func equals(other: Self) -> Bool }
            struct Box[T] { var value: T }
            extend Box[T] where T: Equatable {
                func hasSameValue(other: Box[T]) -> Bool { return self.value.equals(other.value); }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn extension_where_clause_not_satisfied() {
        Test::new(
            r#"module Test
            protocol Equatable { func equals(other: Self) -> Bool }
            struct NotEquatable { }
            struct Box[T] { var value: T }
            extend Box[T] where T: Equatable {
                func hasSameValue(other: Box[T]) -> Bool { return self.value.equals(other.value); }
            }
            func test() -> Bool {
                let b1 = Box[NotEquatable](value: NotEquatable());
                let b2 = Box[NotEquatable](value: NotEquatable());
                return b1.hasSameValue(b2);
            }
        "#,
        )
        .expect(HasError("hasSameValue"));
    }

    #[test]
    fn extension_inherits_struct_constraints() {
        Test::new(
            r#"module Test
            protocol Comparable { func lessThan(other: Self) -> Bool }
            struct SortedBox[T] where T: Comparable { var value: T }
            extend SortedBox[T] {
                func isLessThan(other: SortedBox[T]) -> Bool { return self.value.lessThan(other.value); }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn extension_adds_additional_constraint() {
        Test::new(
            r#"module Test
            protocol Comparable { func lessThan(other: Self) -> Bool }
            protocol Hashable { func hash() -> Int }
            struct SortedBox[T] where T: Comparable { var value: T }
            extend SortedBox[T] where T: Hashable {
                func getHash() -> Int { return self.value.hash(); }
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod conflicts {
    use super::*;

    #[test]
    fn duplicate_method_same_specificity_error() {
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            extend Point {
                func foo() -> Int { return 1; }
            }
            extend Point {
                func foo() -> Int { return 2; }
            }
        "#,
        )
        .expect(HasError("duplicate"));
    }

    #[test]
    fn different_specificity_no_conflict() {
        Test::new(
            r#"module Test
            struct Box[T] { var value: T }
            extend Box[T] {
                func describe() -> String { return "generic"; }
            }
            extend Box[Int] {
                func describe() -> String { return "int"; }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn struct_method_vs_extension_method() {
        Test::new(
            r#"module Test
            struct Point {
                var x: Int; var y: Int
                func sum() -> Int { return self.x + self.y; }
            }
            extend Point {
                func sum() -> Int { return 0; }
            }
        "#,
        )
        .expect(HasError("duplicate"));
    }
}

mod errors {
    use super::*;

    #[test]
    fn extend_unknown_type() {
        Test::new(
            r#"module Test
            extend Unknown { func foo() { } }
        "#,
        )
        .expect(HasError("Unknown"));
    }

    #[test]
    fn extend_primitive() {
        Test::new(
            r#"module Test
            extend Int {
                func doubled() -> Int { return self * 2; }
            }
        "#,
        )
        .expect(HasError("cannot extend"));
    }

    #[test]
    fn extend_type_alias() {
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            type MyPoint = Point;
            extend MyPoint { func foo() { } }
        "#,
        )
        .expect(HasError("cannot extend"));
    }

    #[test]
    fn wrong_type_param_count() {
        Test::new(
            r#"module Test
            struct Box[T] { var value: T }
            extend Box[T, U] { func foo() { } }
        "#,
        )
        .expect(HasError("type parameter"));
    }

    #[test]
    fn extension_type_param_not_in_scope() {
        Test::new(
            r#"module Test
            struct Box[T] { var value: T }
            extend Box[T] {
                func withU() -> U { return self.value; }
            }
        "#,
        )
        .expect(HasError("U"));
    }
}

mod visibility {
    use super::*;

    #[test]
    fn public_extension_method() {
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            extend Point {
                public func sum() -> Int { return self.x + self.y; }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn private_extension_method() {
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            extend Point {
                private func internalSum() -> Int { return self.x + self.y; }
                func doubleSum() -> Int { return self.internalSum() * 2; }
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod self_type {
    use super::*;

    #[test]
    fn extension_method_uses_self() {
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            extend Point {
                func clone() -> Self { return Point(x: self.x, y: self.y); }
            }
            func test() -> Point {
                let p = Point(x: 1, y: 2);
                return p.clone();
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn extension_method_self_param() {
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            extend Point {
                func add(other: Self) -> Self { return Point(x: self.x + other.x, y: self.y + other.y); }
            }
            func test() -> Point {
                let p1 = Point(x: 1, y: 2);
                let p2 = Point(x: 3, y: 4);
                return p1.add(p2);
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod static_methods {
    use super::*;

    #[test]
    fn extension_static_method() {
        Test::new(
            r#"module Test
            struct Point { var x: Int; var y: Int }
            extend Point {
                static func origin() -> Point { return Point(x: 0, y: 0); }
            }
            func test() -> Point { return Point.origin(); }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn generic_extension_static_method() {
        Test::new(
            r#"module Test
            struct Box[T] { var value: T }
            extend Box[Int] {
                static func zero() -> Box[Int] { return Box[Int](value: 0); }
            }
            func test() -> Box[Int] { return Box[Int].zero(); }
        "#,
        )
        .expect(Compiles);
    }
}

mod constraint_inference {
    use super::*;

    #[test]
    fn extension_associated_type_resolution() {
        Test::new(
            r#"module Test
            protocol Mapper {
                type Source;
                func map(s: Source)
            }
            struct Box[T] { var value: T }
            extend Box[T] where T: Mapper {
                func doMap(s: T.Source) {
                    self.value.map(s)
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn extension_equality_constraint_enforcement() {
        Test::new(
            r#"module Test
            protocol Mapper {
                type Source;
                func map(s: Source)
            }
            struct Box[T] { var value: T }
            extend Box[T] where T: Mapper, T.Source = Int {
                func mapString(s: String) {
                    // Should fail: T.Source is Int, but s is String
                    self.value.map(s)
                }
            }
        "#,
        )
        .expect(HasError("type mismatch"));
    }
}

mod future_features {
    use super::*;

    #[test]
    fn extend_enum() {
        Test::new(
            r#"module Test
            enum Color {
                case Red
                case Green
                case Blue
            }
            extend Color {
                func isRed() -> Bool { return true; }
            }
        "#,
        )
        .expect(Compiles);
    }
}
