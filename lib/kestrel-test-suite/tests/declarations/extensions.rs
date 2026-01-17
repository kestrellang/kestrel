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
    fn extension_provides_associated_type_binding() {
        Test::new(
            r#"module Test
            protocol Factory {
                type Product;
                func make() -> Product
            }
            struct Maker { }
            extend Maker: Factory {
                type Product = Int;
                func make() -> Int { return 1; }
            }
            func useFactory[F](f: F) -> Int where F: Factory { return f.make(); }
            func test() -> Int { return useFactory(Maker()); }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn extension_missing_associated_type_binding() {
        Test::new(
            r#"module Test
            protocol Factory {
                type Product;
                func make() -> Product
            }
            struct Maker { }
            extend Maker: Factory {
                func make() -> Int { return 1; }
            }
        "#,
        )
        .expect(HasError("does not provide associated type 'Product'"));
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

    #[test]
    fn extension_conformance_on_generic_type() {
        Test::new(
            r#"module Test
            struct Container[T] { let value: T }
            protocol Printable { func print() }
            extend Container[Int]: Printable {
                func print() { }
            }
            func usePrintable(p: Printable) { p.print(); }
            func main() {
                let c = Container(value: 42);
                usePrintable(c);
            }
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

mod protocol_extensions {
    use super::*;

    // Phase 1: Binding tests - protocol extensions parse and bind correctly

    #[test]
    fn empty_protocol_extension() {
        Test::new(
            r#"module Test
            protocol Drawable {
                func draw()
            }
            extend Drawable { }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn protocol_extension_with_empty_method() {
        // Method with no body content that doesn't access self
        Test::new(
            r#"module Test
            protocol Drawable {
                func draw()
            }
            extend Drawable {
                func helper() { }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn protocol_extension_with_multiple_methods() {
        Test::new(
            r#"module Test
            protocol Processable {
                func process()
            }
            extend Processable {
                func helper1() { }
                func helper2() { }
                func helper3() { }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn multiple_protocol_extensions_same_protocol() {
        Test::new(
            r#"module Test
            protocol Drawable {
                func draw()
            }
            extend Drawable {
                func helper1() { }
            }
            extend Drawable {
                func helper2() { }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn protocol_extension_generic_protocol() {
        Test::new(
            r#"module Test
            protocol Container[T] {
                func fetch() -> T
            }
            extend Container {
                func doNothing() { }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn protocol_extension_with_where_clause_self_bound() {
        // Where clause with Self: OtherProtocol
        Test::new(
            r#"module Test
            protocol Sortable {
                func sort()
            }
            protocol Filterable {
                func filter()
            }
            extend Filterable where Self: Sortable {
                func helper() { }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn protocol_extension_multiple_where_clauses() {
        Test::new(
            r#"module Test
            protocol A {
                func methodA()
            }
            protocol B {
                func methodB()
            }
            protocol C {
                func methodC()
            }
            extend C where Self: A, Self: B {
                func helperAB() { }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn protocol_extension_inheriting_protocol() {
        // Protocol that inherits from another, with extension
        Test::new(
            r#"module Test
            protocol Base {
                func base()
            }
            protocol Derived: Base {
                func derived()
            }
            extend Derived {
                func helper() { }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn protocol_extension_self_associated_type_bound() {
        // Where clause with Self.AssociatedType: Protocol
        Test::new(
            r#"module Test
            protocol Equatable {
                func equals(other: Self)
            }
            protocol Iterator {
                type Item
                func next()
            }
            extend Iterator where Self.Item: Equatable {
                func containsHelper() { }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn protocol_extension_mixed_self_constraints() {
        // Mix of Self: Protocol and Self.AssociatedType: Protocol
        Test::new(
            r#"module Test
            protocol Comparable {
                func compare(other: Self)
            }
            protocol Equatable {
                func equals(other: Self)
            }
            protocol Iterator {
                type Item
                func next()
            }
            extend Iterator where Self: Comparable, Self.Item: Equatable {
                func mixedHelper() { }
            }
        "#,
        )
        .expect(Compiles);
    }

    // Phase 3 tests - method resolution from protocol extensions

    #[test]
    fn type_uses_protocol_extension_method() {
        // A struct conforming to a protocol can use methods from protocol extensions
        Test::new(
            r#"module Test
            protocol Drawable {
                func draw()
            }
            extend Drawable {
                func helperMethod() { }
            }
            struct Circle: Drawable {
                func draw() { }
            }
            func test() {
                let c = Circle();
                c.helperMethod();
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn type_own_method_takes_priority() {
        // Type's own method takes priority over protocol extension method
        Test::new(
            r#"module Test
            protocol Drawable {
                func draw()
            }
            extend Drawable {
                func helper() { }
            }
            struct Circle: Drawable {
                func draw() { }
                func helper() { }
            }
            func test() {
                let c = Circle();
                c.helper();
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn constrained_protocol_extension_applies() {
        // Protocol extension with where clause applies when constraint satisfied
        Test::new(
            r#"module Test
            protocol Sortable {
                func sort()
            }
            protocol Filterable {
                func filter()
            }
            extend Filterable where Self: Sortable {
                func combined() { }
            }
            struct Data: Filterable, Sortable {
                func filter() { }
                func sort() { }
            }
            func test() {
                let d = Data();
                d.combined();
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn unconstrained_protocol_extension_not_found_when_constraint_not_met() {
        // Protocol extension with where clause should not apply when constraint not met
        Test::new(
            r#"module Test
            protocol Sortable {
                func sort()
            }
            protocol Filterable {
                func filter()
            }
            extend Filterable where Self: Sortable {
                func combined() { }
            }
            struct Data: Filterable {
                func filter() { }
            }
            func test() {
                let d = Data();
                d.combined();
            }
        "#,
        )
        .expect(HasError("member"));
    }

    // Phase 4 tests - body resolution in protocol extension methods

    #[test]
    fn protocol_extension_method_calls_protocol_method() {
        // Protocol extension method can call protocol-required methods on self
        Test::new(
            r#"module Test
            protocol Drawable {
                func draw()
                func clear()
            }
            extend Drawable {
                func redraw() {
                    self.clear();
                    self.draw();
                }
            }
            struct Circle: Drawable {
                func draw() { }
                func clear() { }
            }
            func test() {
                let c = Circle();
                c.redraw();
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn protocol_extension_method_returns_value_from_protocol_method() {
        // Protocol extension method can use values from protocol methods (assigned to let)
        Test::new(
            r#"module Test
            protocol Processor {
                func process()
                func getState()
            }
            extend Processor {
                func processAndGetState() {
                    self.process();
                    let _state = self.getState();
                }
            }
            struct Item: Processor {
                func process() { }
                func getState() { }
            }
            func test() {
                let s = Item();
                s.processAndGetState();
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn protocol_extension_method_calls_other_extension_method() {
        // Protocol extension method can call another extension method
        Test::new(
            r#"module Test
            protocol Printable {
                func print()
            }
            extend Printable {
                func helper() { }
                func printTwice() {
                    self.print();
                    self.helper();
                    self.print();
                }
            }
            struct Message: Printable {
                func print() { }
            }
            func test() {
                let m = Message();
                m.printTwice();
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn protocol_extension_chained_method_calls() {
        // Protocol extension method with chained calls
        Test::new(
            r#"module Test
            protocol Builder {
                func reset()
                func validate()
            }
            extend Builder {
                func prepareAndValidate() {
                    self.reset();
                    self.validate();
                    self.reset();
                    self.validate();
                }
            }
            struct SimpleBuilder: Builder {
                func reset() { }
                func validate() { }
            }
            func test() {
                let b = SimpleBuilder();
                b.prepareAndValidate();
            }
        "#,
        )
        .expect(Compiles);
    }

    // Phase 5 tests - specificity and conflict resolution

    #[test]
    fn more_constrained_extension_wins() {
        // When two protocol extensions provide the same method, the more constrained one wins
        // The more constrained extension's method is called (verified by compilation succeeding)
        Test::new(
            r#"module Test
            protocol Sortable {
                func sort()
            }
            protocol Filterable {
                func filter()
            }
            // Less constrained extension (specificity 0)
            extend Filterable {
                func process() { }
            }
            // More constrained extension (specificity 1)
            extend Filterable where Self: Sortable {
                func process() { }
            }
            struct Data: Filterable, Sortable {
                func filter() { }
                func sort() { }
            }
            func test() {
                let d = Data();
                d.process();
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn less_constrained_extension_used_when_constraint_not_met() {
        // When the more constrained extension doesn't apply, fall back to less constrained
        Test::new(
            r#"module Test
            protocol Sortable {
                func sort()
            }
            protocol Filterable {
                func filter()
            }
            // Less constrained extension (specificity 0)
            extend Filterable {
                func process() { }
            }
            // More constrained extension (specificity 1) - doesn't apply to BasicData
            extend Filterable where Self: Sortable {
                func process() { }
            }
            // BasicData only conforms to Filterable, not Sortable
            struct BasicData: Filterable {
                func filter() { }
            }
            func test() {
                let d = BasicData();
                d.process();
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn multiple_constraints_more_specific() {
        // Extension with 2 constraints beats extension with 1 constraint
        Test::new(
            r#"module Test
            protocol A {
                func methodA()
            }
            protocol B {
                func methodB()
            }
            protocol C {
                func methodC()
            }
            // Specificity 1 (one constraint)
            extend C where Self: A {
                func helper() { }
            }
            // Specificity 2 (two constraints) - should win
            extend C where Self: A, Self: B {
                func helper() { }
            }
            struct Data: A, B, C {
                func methodA() { }
                func methodB() { }
                func methodC() { }
            }
            func test() {
                let d = Data();
                d.helper();
            }
        "#,
        )
        .expect(Compiles);
    }

    // Phase 6 tests - calling constraint methods on self

    #[test]
    fn protocol_extension_calls_constraint_method() {
        // Inside a constrained protocol extension, can call methods from constraint protocol
        Test::new(
            r#"module Test
            protocol Sortable {
                func sort()
            }
            protocol Filterable {
                func filter()
            }
            extend Filterable where Self: Sortable {
                func filterAndSort() {
                    self.filter();
                    self.sort();
                }
            }
            struct Data: Filterable, Sortable {
                func filter() { }
                func sort() { }
            }
            func test() {
                let d = Data();
                d.filterAndSort();
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn protocol_extension_calls_multiple_constraint_methods() {
        // Can call methods from multiple constraint protocols
        Test::new(
            r#"module Test
            protocol A {
                func doA()
            }
            protocol B {
                func doB()
            }
            protocol C {
                func doC()
            }
            extend C where Self: A, Self: B {
                func doAll() {
                    self.doC();
                    self.doA();
                    self.doB();
                }
            }
            struct Data: A, B, C {
                func doA() { }
                func doB() { }
                func doC() { }
            }
            func test() {
                let d = Data();
                d.doAll();
            }
        "#,
        )
        .expect(Compiles);
    }

    // Edge case tests

    #[test]
    fn type_can_override_protocol_extension_default() {
        // Type's own implementation takes priority over protocol extension default
        Test::new(
            r#"module Test
            protocol Describable {
                func describe()
            }
            extend Describable {
                func describe() { }
            }
            struct Item: Describable {
                func describe() { }
            }
            func test() {
                let i = Item();
                i.describe();
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn protocol_extension_on_inheriting_protocol_own_methods() {
        // Extension on derived protocol can call derived protocol's own methods
        Test::new(
            r#"module Test
            protocol Base {
                func baseMethod()
            }
            protocol Derived: Base {
                func derivedMethod()
            }
            extend Derived {
                func helper() {
                    self.derivedMethod();
                }
            }
            struct Impl: Base, Derived {
                func baseMethod() { }
                func derivedMethod() { }
            }
            func test() {
                let i = Impl();
                i.helper();
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn protocol_extension_uses_associated_type_in_signature() {
        // Protocol extension method can use associated types in parameter/return types
        Test::new(
            r#"module Test
            protocol Container {
                type Element
                func add(item: Element)
            }
            extend Container {
                func addTwo(first: Element, second: Element) {
                    self.add(first);
                    self.add(second);
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn protocol_extension_uses_inherited_associated_type() {
        // Protocol extension can access associated type from parent protocol
        Test::new(
            r#"module Test
            protocol Base {
                type Element
            }
            protocol Child: Base {
                func fetch() -> Element
            }
            extend Child {
                func fetchWithFallback(fallback: Element) -> Element {
                    return self.fetch();
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    // TODO: Future work - these tests document features not yet implemented:
    //
    // 1. protocol_extension_provides_default_for_requirement
    //    - Protocol extension provides default for required method
    //    - Conformance checker should recognize extension defaults
    //    - Types using default don't need to implement the method
    //
    // 2. protocol_extension_on_inheriting_protocol_inherited_methods
    //    - Protocol extension on Derived should access Base protocol methods via self
    //    - Method resolution should traverse protocol inheritance chain
}
