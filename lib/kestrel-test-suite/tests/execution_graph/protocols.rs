//! Protocol and Witness MIR tests.
//!
//! Tests for protocol and witness lowering including:
//! - Protocol definitions with methods and associated types
//! - Protocol inheritance
//! - Witness generation from conformances
//! - Generic witnesses
//! - Witness method calls on type parameters

use kestrel_test_suite::mir::*;
use kestrel_test_suite::*;

// ============================================================================
// BASIC PROTOCOL LOWERING
// ============================================================================

mod basic_protocol {
    use super::*;

    #[test]
    fn empty_protocol_lowers_to_mir() {
        Test::new(
            r#"
            module Test

            protocol Marker { }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_protocol("Test.Marker").has_method_count(0));
    }

    #[test]
    fn protocol_with_single_method() {
        Test::new(
            r#"
            module Test

            protocol Hashable {
                func hash() -> lang.i64
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_protocol("Test.Hashable")
                .has_method("hash")
                .has_method_count(1),
        );
    }

    #[test]
    fn protocol_with_multiple_methods() {
        Test::new(
            r#"
            module Test

            protocol Comparable {
                func lessThan(other: lang.i64) -> lang.i1
                func equals(other: lang.i64) -> lang.i1
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_protocol("Test.Comparable")
                .has_method("lessThan")
                .has_method("equals")
                .has_method_count(2),
        );
    }

    #[test]
    fn public_protocol() {
        Test::new(
            r#"
            module Test

            public protocol Drawable { }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_protocol("Test.Drawable"));
    }
}

// ============================================================================
// ASSOCIATED TYPES IN PROTOCOLS
// ============================================================================

mod associated_types {
    use super::*;

    #[test]
    fn protocol_with_associated_type() {
        Test::new(
            r#"
            module Test

            protocol Iterator {
                type Item;
                func next() -> Item
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_protocol("Test.Iterator")
                .has_associated_type("Item")
                .has_method("next"),
        );
    }

    #[test]
    fn protocol_with_multiple_associated_types() {
        Test::new(
            r#"
            module Test

            protocol Dictionary {
                type Key;
                type Value;
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_protocol("Test.Dictionary")
                .has_associated_type("Key")
                .has_associated_type("Value"),
        );
    }

    #[test]
    fn associated_type_with_default() {
        Test::new(
            r#"
            module Test

            protocol Parser {
                type Output = lang.str;
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_protocol("Test.Parser").has_associated_type("Output"));
    }
}

// ============================================================================
// GENERIC PROTOCOLS
// ============================================================================

mod generic_protocol {
    use super::*;

    #[test]
    fn generic_protocol_preserves_type_params() {
        Test::new(
            r#"
            module Test

            protocol Container[T] { }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_protocol("Test.Container").has_type_params(1));
    }

    #[test]
    fn generic_protocol_with_method_using_type_param() {
        Test::new(
            r#"
            module Test

            protocol Container[T] {
                func read() -> T
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_protocol("Test.Container")
                .has_type_params(1)
                .has_method("read"),
        );
    }

    #[test]
    fn generic_protocol_multiple_type_params() {
        Test::new(
            r#"
            module Test

            protocol Mapping[K, V] {
                func read(key: K) -> V
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_protocol("Test.Mapping").has_type_params(2));
    }
}

// ============================================================================
// SELF TYPE IN PROTOCOLS
// ============================================================================

mod self_type {
    use super::*;

    #[test]
    fn protocol_method_with_self_param() {
        Test::new(
            r#"
            module Test

            protocol Equatable {
                func eq(other: Self) -> lang.i1
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_protocol("Test.Equatable").has_method("eq"));
    }

    #[test]
    fn protocol_method_returns_self() {
        Test::new(
            r#"
            module Test

            protocol Cloneable {
                func clone() -> Self
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_protocol("Test.Cloneable").has_method("clone"));
    }

    #[test]
    #[ignore]
    fn protocol_method_with_self_in_array() {
        Test::new(
            r#"
            module Test

            protocol Collection {
                func getAll() -> [Self]
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_protocol("Test.Collection").has_method("getAll"));
    }
}

// ============================================================================
// RECEIVER KINDS
// ============================================================================

mod receiver_kinds {
    use super::*;

    #[test]
    fn static_method_in_protocol() {
        Test::new(
            r#"
            module Test

            protocol Factory {
                static func create() -> lang.i64
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_protocol("Test.Factory").has_method("create"));
    }

    #[test]
    fn mutating_method_in_protocol() {
        Test::new(
            r#"
            module Test

            protocol Incrementable {
                mutating func increment()
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_protocol("Test.Incrementable").has_method("increment"));
    }

    #[test]
    fn consuming_method_in_protocol() {
        Test::new(
            r#"
            module Test

            protocol Disposable {
                consuming func dispose()
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_protocol("Test.Disposable").has_method("dispose"));
    }
}

// ============================================================================
// PROTOCOL INHERITANCE
// ============================================================================

mod protocol_inheritance {
    use super::*;

    #[test]
    fn protocol_inherits_single_protocol() {
        Test::new(
            r#"
            module Test

            protocol Drawable { }
            protocol Shape: Drawable { }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_protocol("Test.Drawable"))
        .expect(Mir::mir_protocol("Test.Shape"));
    }

    #[test]
    fn protocol_inherits_multiple_protocols() {
        Test::new(
            r#"
            module Test

            protocol Drawable { }
            protocol Clickable { }
            protocol Widget: Drawable, Clickable { }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_protocol("Test.Widget"));
    }

    #[test]
    fn protocol_with_inherited_and_own_methods() {
        Test::new(
            r#"
            module Test

            protocol Drawable {
                func draw()
            }
            protocol Shape: Drawable {
                func area() -> lang.i64
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_protocol("Test.Drawable").has_method("draw"))
        .expect(Mir::mir_protocol("Test.Shape").has_method("area"));
    }
}

// ============================================================================
// WITNESS GENERATION FROM STRUCT CONFORMANCE
// ============================================================================

mod witness_from_struct {
    use super::*;

    #[test]
    fn struct_conformance_generates_witness() {
        Test::new(
            r#"
            module Test

            protocol Drawable {
                func draw()
            }

            struct Circle: Drawable {
                func draw() { }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_witness("Test.Circle", "Test.Drawable").has_method("draw"));
    }

    #[test]
    fn witness_with_multiple_methods() {
        Test::new(
            r#"
            module Test

            protocol Comparable {
                func lessThan(other: lang.i64) -> lang.i1
                func equals(other: lang.i64) -> lang.i1
            }

            struct Number: Comparable {
                func lessThan(other: lang.i64) -> lang.i1 { true }
                func equals(other: lang.i64) -> lang.i1 { false }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_witness("Test.Number", "Test.Comparable")
                .has_method("lessThan")
                .has_method("equals")
                .has_method_count(2),
        );
    }

    #[test]
    fn witness_with_associated_type() {
        Test::new(
            r#"
            module Test

            protocol Iterator {
                type Item;
                func next() -> Item
            }

            struct IntIterator: Iterator {
                type Item = lang.i64;
                func next() -> lang.i64 { 0 }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_witness("Test.IntIterator", "Test.Iterator")
                .has_associated_type("Item")
                .has_method("next"),
        );
    }

    #[test]
    fn witness_from_multiple_conformances() {
        Test::new(
            r#"
            module Test

            protocol Drawable {
                func draw()
            }

            protocol Clickable {
                func onClick()
            }

            struct Button: Drawable, Clickable {
                func draw() { }
                func onClick() { }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_witness("Test.Button", "Test.Drawable"))
        .expect(Mir::mir_witness("Test.Button", "Test.Clickable"))
        .expect(Mir::witness_count(2));
    }
}

// ============================================================================
// WITNESS GENERATION FROM EXTENSION CONFORMANCE
// ============================================================================

mod witness_from_extension {
    use super::*;

    #[test]
    fn extension_conformance_generates_witness() {
        Test::new(
            r#"
            module Test

            protocol Hashable {
                func hash() -> lang.i64
            }

            struct Point { }

            extend Point: Hashable {
                func hash() -> lang.i64 { 42 }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_witness("Test.Point", "Test.Hashable"));
    }
}

// ============================================================================
// GENERIC WITNESS
// ============================================================================

mod generic_witness {
    use super::*;

    #[test]
    fn generic_struct_witness() {
        Test::new(
            r#"
            module Test

            protocol Container {
                type Item;
            }

            struct Box[T]: Container {
                type Item = T;
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_witness("Test.Box[T]", "Test.Container"));
    }

    #[test]
    fn generic_struct_witness_with_method() {
        Test::new(
            r#"
            module Test

            protocol Getter {
                func read() -> lang.i64
            }

            struct Box[T]: Getter {
                func read() -> lang.i64 { 42 }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_witness("Test.Box[T]", "Test.Getter").has_method("read"));
    }
}

// ============================================================================
// INHERITED PROTOCOL WITNESS
// ============================================================================

mod inherited_witness {
    use super::*;

    #[test]
    fn struct_satisfies_inherited_methods() {
        // When S conforms to B (which inherits from A), S must also explicitly conform to A
        Test::new(
            r#"
            module Test

            protocol A {
                func a()
            }

            protocol B: A {
                func b()
            }

            struct S: A, B {
                func a() { }
                func b() { }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        // Should have witnesses for both A and B
        .expect(Mir::mir_witness("Test.S", "Test.B"))
        .expect(Mir::mir_witness("Test.S", "Test.A"));
    }

    #[test]
    fn missing_parent_conformance_is_error() {
        // Conforming to B without also conforming to A should be an error
        Test::new(
            r#"
            module Test

            protocol A {
                func a()
            }

            protocol B: A {
                func b()
            }

            struct S: B {
                func a() { }
                func b() { }
            }
        "#,
        )
        .expect(HasError("conforms to 'B' but not its parent protocol 'A'"));
    }
}

// ============================================================================
// WITNESS METHOD CALLS (Type Parameter Calls)
// ============================================================================

mod witness_method_calls {
    use super::*;

    #[test]
    fn instance_method_on_type_parameter() {
        // a.add(b) where a: T, T: Add
        Test::new(
            r#"
            module Test

            protocol Add {
                func add(other: Self) -> Self
            }

            func addThem[T](a: T, b: T) -> T where T: Add {
                return a.add(b)
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Test.addThem$a$b")
                .has_type_params(1)
                .calls_witness("Test.Add", "add"),
        );
    }

    #[test]
    fn static_method_on_type_parameter() {
        // T.create() where T: Factory
        Test::new(
            r#"
            module Test

            protocol Factory {
                static func create() -> Self
            }

            func make[T]() -> T where T: Factory {
                return T.create()
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Test.make")
                .has_type_params(1)
                .calls_witness("Test.Factory", "create"),
        );
    }

    #[test]
    fn init_on_type_parameter() {
        // T() where T: Factory
        Test::new(
            r#"
            module Test

            protocol Factory {
                init()
            }

            func make[T]() -> T where T: Factory {
                return T()
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Test.make")
                .has_type_params(1)
                .calls_witness("Test.Factory", "init"),
        );
    }

    #[test]
    fn init_with_arguments_on_type_parameter() {
        // T(v) where T: Factory
        Test::new(
            r#"
            module Test

            protocol Factory {
                init(value: lang.i64)
            }

            func make[T](v: lang.i64) -> T where T: Factory {
                return T(v)
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Test.make$v")
                .has_type_params(1)
                .calls_witness("Test.Factory", "init"),
        );
    }

    #[test]
    fn method_with_arguments_on_type_parameter() {
        // a.process(x, y) where a: T, T: Processor
        Test::new(
            r#"
            module Test

            protocol Processor {
                func process(x: lang.i64, y: lang.i64) -> lang.i64
            }

            func run[T](proc: T, a: lang.i64, b: lang.i64) -> lang.i64 where T: Processor {
                return proc.process(a, b)
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Test.run$proc$a$b")
                .has_type_params(1)
                .calls_witness("Test.Processor", "process"),
        );
    }

    #[test]
    fn multiple_bounds_uses_correct_protocol() {
        // When T has multiple bounds, the witness should reference the correct protocol
        Test::new(
            r#"
            module Test

            protocol Add {
                func add(other: Self) -> Self
            }

            protocol Mul {
                func mul(other: Self) -> Self
            }

            func compute[T](a: T, b: T) -> T where T: Add and Mul {
                let sum = a.add(b);
                return sum.mul(b)
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Test.compute$a$b")
                .has_type_params(1)
                .calls_witness("Test.Add", "add")
                .calls_witness("Test.Mul", "mul"),
        );
    }

    #[test]
    fn static_method_with_arguments() {
        // T.fromInt(42) where T: Convertible
        Test::new(
            r#"
            module Test

            protocol Convertible {
                static func fromInt(value: lang.i64) -> Self
            }

            func convert[T](n: lang.i64) -> T where T: Convertible {
                return T.fromInt(n)
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Test.convert$n")
                .has_type_params(1)
                .calls_witness("Test.Convertible", "fromInt"),
        );
    }

    #[test]
    fn mutating_method_on_type_parameter() {
        // a.increment() where T: Counter, mutating func
        Test::new(
            r#"
            module Test

            protocol Counter {
                mutating func increment()
            }

            func bump[T](a: T) where T: Counter {
                var x = a;
                x.increment()
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Test.bump$a")
                .has_type_params(1)
                .calls_witness("Test.Counter", "increment"),
        );
    }

    #[test]
    fn static_method_on_associated_type() {
        // T.Item.create() where T: Container, Container.Item: Factory
        Test::new(
            r#"
            module Test

            protocol Factory {
                static func create() -> Self
            }

            protocol Container {
                type Item: Factory;
            }

            func makeItem[T]() -> T.Item where T: Container {
                return T.Item.create()
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Test.makeItem")
                .has_type_params(1)
                .calls_witness("Test.Factory", "create"),
        );
    }
}
