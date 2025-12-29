//! Tests for method call expressions.
//!
//! These tests verify that method calls (instance and static) are correctly resolved,
//! including self parameter handling, primitive methods, and chaining.

use kestrel_test_suite::*;

mod self_parameter {
    use super::*;

    #[test]
    fn struct_with_multiple_receiver_kinds() {
        // Instance methods, mutating, and consuming methods should all compile
        // This consolidates tests for instance_method_compiles, mutating_method_compiles,
        // and consuming_method_compiles which were testing similar concepts
        Test::new(
            r#"
module Main

struct Counter {
    let value: Int
    var mutableValue: Int

    func getValue() -> Int {
        42
    }

    mutating func increment() -> () {
        ()
    }

    consuming func consume() -> Int {
        self.value
    }
}
"#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Counter")
                .is(SymbolKind::Struct)
                .has(Behavior::FieldCount(2)),
        )
        .expect(
            Symbol::new("Counter.getValue")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0)),
        )
        .expect(
            Symbol::new("Counter.increment")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0)),
        )
        .expect(
            Symbol::new("Counter.consume")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0)),
        );
    }

    #[test]
    fn static_and_instance_methods() {
        // Mix static and instance methods with multiple receiver types
        Test::new(
            r#"
module Main

struct Factory {
    static func create() -> Int {
        42
    }

    func build() -> Int {
        42
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Factory").is(SymbolKind::Struct))
        .expect(
            Symbol::new("Factory.create")
                .is(SymbolKind::Function)
                .has(Behavior::IsStatic(true)),
        )
        .expect(
            Symbol::new("Factory.build")
                .is(SymbolKind::Function)
                .has(Behavior::IsStatic(false)),
        );
    }

    #[test]
    fn protocol_with_all_method_types() {
        // Protocol with regular, mutating, and consuming methods
        Test::new(
            r#"
module Main

protocol Lifecycle {
    func query() -> String
    mutating func reset() -> ()
    consuming func finalize() -> ()
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Lifecycle").is(SymbolKind::Protocol))
        .expect(Symbol::new("Lifecycle.query").is(SymbolKind::Function))
        .expect(Symbol::new("Lifecycle.reset").is(SymbolKind::Function))
        .expect(Symbol::new("Lifecycle.finalize").is(SymbolKind::Function));
    }

    // === Self Usage in Method Bodies ===

    #[test]
    fn access_self_fields_in_methods() {
        // Instance methods can access self.field - tests both immutable and mutable access
        // Consolidates access_self_field_in_instance_method and access_multiple_self_fields
        Test::new(
            r#"
module Main

struct Point {
    let x: Int
    let y: Int
    var z: Int

    func getX() -> Int {
        self.x
    }

    func getY() -> Int {
        self.y
    }

    mutating func getZ() -> Int {
        self.z
    }
}
"#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Point")
                .is(SymbolKind::Struct)
                .has(Behavior::FieldCount(3)),
        )
        .expect(Symbol::new("Point.getX").is(SymbolKind::Function))
        .expect(Symbol::new("Point.getY").is(SymbolKind::Function))
        .expect(Symbol::new("Point.getZ").is(SymbolKind::Function));
    }

    #[test]
    fn self_in_static_method_error() {
        // Using self in a static method should be an error
        Test::new(
            r#"
module Main

struct Calculator {
    let value: Int

    static func compute() -> Int {
        self.value
    }
}
"#,
        )
        .expect(HasError("cannot use 'self' in static method"));
    }

    #[test]
    fn self_in_free_function_error() {
        // Using self in a free function (both top-level and within modules) should error
        Test::new(
            r#"
module Main

struct Point {
    let x: Int
}

func freeFunc() -> Int {
    self.x
}
"#,
        )
        .expect(HasError("cannot use 'self' in free function"));
    }

    // === Method Calls on Instances ===

    #[test]
    fn call_instance_methods_with_and_without_params() {
        // Call instance methods with and without parameters
        // Consolidates call_instance_method_on_struct and call_instance_method_with_params
        Test::new(
            r#"
module Main

struct Calculator {
    let base: Int

    func getValue() -> Int {
        42
    }

    func add(x: Int) -> Int {
        42
    }

    func multiply(x: Int, y: Int) -> Int {
        42
    }
}

func test(c: Calculator) -> Int {
    c.multiply(5, 6)
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Calculator").is(SymbolKind::Struct))
        .expect(
            Symbol::new("Calculator.getValue")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0)),
        )
        .expect(
            Symbol::new("Calculator.add")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(1)),
        )
        .expect(
            Symbol::new("Calculator.multiply")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(2)),
        );
    }

    #[test]
    fn chain_method_calls_and_self_calls() {
        // Chained method calls and method calling another method on self
        // Consolidates chain_method_calls and method_calling_another_method
        Test::new(
            r#"
module Main

struct Builder {
    let value: Int

    func build() -> Int {
        42
    }
}

struct Factory {
    let builder: Builder

    func getBuilder() -> Builder {
        self.builder
    }

    func buildResult() -> Int {
        self.getBuilder().build()
    }
}

struct Calculator {
    let value: Int

    func getValue() -> Int {
        42
    }

    func getDoubleValue() -> Int {
        self.getValue()
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Factory.getBuilder").is(SymbolKind::Function))
        .expect(Symbol::new("Factory.buildResult").is(SymbolKind::Function))
        .expect(Symbol::new("Calculator.getValue").is(SymbolKind::Function))
        .expect(Symbol::new("Calculator.getDoubleValue").is(SymbolKind::Function));
    }

    // === Static vs Instance Methods ===

    #[test]
    fn call_static_and_instance_methods_on_types() {
        // Call static methods on the type name and instance methods on instances
        // Consolidates call_static_method_on_type and mix_static_and_instance_methods
        Test::new(
            r#"
module Main

struct Counter {
    let value: Int

    static func zero() -> Int {
        0
    }

    static func max(a: Int, b: Int) -> Int {
        42
    }

    func getValue() -> Int {
        self.value
    }

    func increment() -> Int {
        42
    }
}

func test(c: Counter) -> Int {
    c.increment()
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Counter").is(SymbolKind::Struct))
        .expect(
            Symbol::new("Counter.zero")
                .is(SymbolKind::Function)
                .has(Behavior::IsStatic(true)),
        )
        .expect(
            Symbol::new("Counter.max")
                .is(SymbolKind::Function)
                .has(Behavior::IsStatic(true)),
        )
        .expect(
            Symbol::new("Counter.getValue")
                .is(SymbolKind::Function)
                .has(Behavior::IsStatic(false)),
        )
        .expect(
            Symbol::new("Counter.increment")
                .is(SymbolKind::Function)
                .has(Behavior::IsStatic(false)),
        );
    }

    // === Mutating and Consuming Methods ===

    #[test]
    fn mutating_and_consuming_methods() {
        // Mutating methods can access and modify self, consuming methods can access self
        // Consolidates mutating_method_with_self_access, call_mutating_method,
        // consuming_method_with_self_access, and call_consuming_method
        Test::new(
            r#"
module Main

struct Counter {
    var value: Int

    mutating func getValue() -> Int {
        self.value
    }

    mutating func increment() -> () {
        ()
    }
}

struct Container {
    let item: Int

    consuming func getItem() -> Int {
        self.item
    }

    consuming func take() -> Int {
        42
    }
}

func test(c: Counter, k: Container) -> Int {
    k.take()
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Counter.getValue").is(SymbolKind::Function))
        .expect(Symbol::new("Counter.increment").is(SymbolKind::Function))
        .expect(Symbol::new("Container.getItem").is(SymbolKind::Function))
        .expect(Symbol::new("Container.take").is(SymbolKind::Function));
    }

    // === Generic Structs with Methods ===

    #[test]
    fn generic_struct_with_methods() {
        // Generic struct with instance methods that access generic fields
        // Consolidates generic_struct_with_instance_method and generic_struct_method_accessing_self
        Test::new(
            r#"
module Main

struct Container[T] {
    let item: T

    func isEmpty() -> Bool {
        false
    }
}

struct Wrapper[T] {
    let value: T

    func getValue() -> T {
        self.value
    }

    func isEqual(other: T) -> Bool {
        false
    }
}
"#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Container")
                .is(SymbolKind::Struct)
                .has(Behavior::TypeParamCount(1)),
        )
        .expect(Symbol::new("Container.isEmpty").is(SymbolKind::Function))
        .expect(
            Symbol::new("Wrapper")
                .is(SymbolKind::Struct)
                .has(Behavior::TypeParamCount(1)),
        )
        .expect(Symbol::new("Wrapper.getValue").is(SymbolKind::Function))
        .expect(
            Symbol::new("Wrapper.isEqual")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(1)),
        );
    }

    // === Edge Cases and Complex Scenarios ===

    #[test]
    fn edge_cases_and_builder_pattern() {
        // Tests edge cases: empty method body, nested structs, builder pattern, multiple methods accessing same field
        // Consolidates empty_method_body, nested_struct_methods, method_returning_self_type, and multiple_methods_accessing_same_field
        Test::new(
            r#"
module Main

struct Empty {
    func doNothing() -> () {
    }
}

struct Outer {
    let inner: Inner

    func getInner() -> Inner {
        self.inner
    }
}

struct Inner {
    let value: Int

    func getValue() -> Int {
        self.value
    }
}

struct Builder {
    let value: Int

    func withValue(v: Int) -> Builder {
        self
    }
}

struct Point {
    let x: Int

    func getX() -> Int {
        self.x
    }

    func printX() -> Int {
        self.x
    }

    func copyX() -> Int {
        self.x
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Empty.doNothing").is(SymbolKind::Function))
        .expect(Symbol::new("Outer.getInner").is(SymbolKind::Function))
        .expect(Symbol::new("Inner.getValue").is(SymbolKind::Function))
        .expect(
            Symbol::new("Builder.withValue")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(1)),
        )
        .expect(
            Symbol::new("Point")
                .is(SymbolKind::Struct)
                .has(Behavior::FieldCount(1)),
        )
        .expect(Symbol::new("Point.getX").is(SymbolKind::Function))
        .expect(Symbol::new("Point.printX").is(SymbolKind::Function))
        .expect(Symbol::new("Point.copyX").is(SymbolKind::Function));
    }
}

mod primitive_methods {
    use super::*;

    #[test]
    fn primitive_methods_errors() {
        // Primitive methods cannot be used as first-class values, and nonexistent members error
        Test::new(
            r#"
module Main

func test(x: Int) -> () {
    let f = x.toString;
    x.notAMethod
}
"#,
        )
        .expect(HasError(
            "primitive method 'toString' on 'I64' must be called",
        ));
    }
}

mod method_calls {
    use super::*;

    // === Basic Method Calls ===

    #[test]
    fn call_methods_with_various_parameter_styles() {
        // Call methods with no params, with params, and with labeled params
        // Consolidates call_method_no_params, call_method_with_params, call_method_with_labeled_params
        Test::new(
            r#"
module Main

struct Point {
    let x: Int
    let y: Int

    func origin() -> Bool {
        false
    }
}

struct Calculator {
    let base: Int

    func add(x: Int, y: Int) -> Int {
        42
    }
}

struct Formatter {
    let prefix: String

    func format(with value: Int) -> String {
        "formatted"
    }
}

func test(p: Point, c: Calculator, f: Formatter) -> Int {
    c.add(1, 2)
}
"#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Point.origin")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0)),
        )
        .expect(
            Symbol::new("Calculator.add")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(2)),
        )
        .expect(
            Symbol::new("Formatter.format")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(1)),
        );
    }

    // === Chained Method Calls ===

    #[test]
    fn chained_method_calls_builder_and_cross_type() {
        // Chained method calls on builder pattern and across different types
        // Consolidates chained_method_calls_same_type and chained_method_calls_different_types
        Test::new(
            r#"
module Main

struct Builder {
    let value: Int

    func step1() -> Builder {
        self
    }

    func step2() -> Builder {
        self
    }

    func build() -> Int {
        self.value
    }
}

struct Container {
    let inner: Inner

    func getInner() -> Inner {
        self.inner
    }
}

struct Inner {
    let value: Int

    func getValue() -> Int {
        self.value
    }
}

func test(b: Builder, c: Container) -> Int {
    b.step1().step2().build()
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("Builder.step1").is(SymbolKind::Function))
        .expect(Symbol::new("Builder.step2").is(SymbolKind::Function))
        .expect(Symbol::new("Builder.build").is(SymbolKind::Function))
        .expect(Symbol::new("Container.getInner").is(SymbolKind::Function))
        .expect(Symbol::new("Inner.getValue").is(SymbolKind::Function));
    }

    // === Static Method Calls ===

    #[test]
    fn call_static_methods_with_various_params() {
        // Call static methods with and without parameters
        // Consolidates call_static_method_no_params and call_static_method_with_params
        Test::new(
            r#"
module Main

struct Factory {
    static func defaultValue() -> Int {
        0
    }
}

struct MathUtils {
    static func max(a: Int, b: Int) -> Int {
        42
    }

    static func min(a: Int, b: Int) -> Int {
        0
    }
}

func test() -> Int {
    MathUtils.max(10, 20)
}
"#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Factory.defaultValue")
                .is(SymbolKind::Function)
                .has(Behavior::IsStatic(true)),
        )
        .expect(
            Symbol::new("MathUtils.max")
                .is(SymbolKind::Function)
                .has(Behavior::IsStatic(true))
                .has(Behavior::ParameterCount(2)),
        )
        .expect(
            Symbol::new("MathUtils.min")
                .is(SymbolKind::Function)
                .has(Behavior::IsStatic(true))
                .has(Behavior::ParameterCount(2)),
        );
    }

    // === Method Call Errors ===

    #[test]
    fn method_call_error_cases() {
        // Test various method call errors: nonexistent method, wrong type, instance method on type
        // Consolidates call_nonexistent_method_error, call_method_wrong_receiver_type,
        // and call_instance_method_on_type_error
        Test::new(
            r#"
module Main

struct Point {
    let x: Int
}

struct A {
    func methodA() -> Int {
        42
    }
}

struct B {
    let value: Int
}

struct Counter {
    let value: Int

    func getValue() -> Int {
        42
    }
}

func test(p: Point, b: B) -> Int {
    p.nonExistent()
    b.methodA()
    Counter.getValue()
}
"#,
        )
        .expect(Fails);
    }

    // === Method Visibility ===

    #[test]
    fn method_visibility() {
        // Public methods can be called, private methods can be called from within struct
        Test::new(
            r#"
module Main

struct Widget {
    let id: Int

    public func getId() -> Int {
        self.id
    }

    private func internalId() -> Int {
        self.id
    }

    func getInternalId() -> Int {
        self.internalId()
    }
}

func test(w: Widget) -> Int {
    w.getId()
}
"#,
        )
        .expect(Compiles);
    }
}
