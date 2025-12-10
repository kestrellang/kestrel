//! Tests for static methods and initializers on type parameters
//!
//! This file tests calling static methods and initializers on type parameters
//! when constrained by protocol bounds.
//!
//! Example:
//! ```kestrel
//! protocol Factory {
//!     init()
//!     static func create() -> Self
//! }
//!
//! func make[T]() -> T where T: Factory {
//!     return T()        // Init on type parameter
//!     return T.create() // Static method on type parameter
//! }
//! ```

use kestrel_test_suite::*;

mod basic_init {
    use super::*;

    #[test]
    fn init_on_type_parameter() {
        // Basic case: T() where T: Protocol with init
        Test::new(
            r#"module Test
            protocol Factory {
                init()
            }
            func make[T]() -> T where T: Factory {
                return T()
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn init_with_arguments() {
        // T(x: 1) with labeled argument
        Test::new(
            r#"module Test
            protocol Factory {
                init(value: Int)
            }
            func make[T](v: Int) -> T where T: Factory {
                return T(value: v)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn init_with_multiple_arguments() {
        // T(a: 1, b: 2) with multiple arguments
        Test::new(
            r#"module Test
            protocol Factory {
                init(x: Int, y: Int)
            }
            func make[T](a: Int, b: Int) -> T where T: Factory {
                return T(x: a, y: b)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn init_overload_resolution() {
        // Multiple inits in protocol, overload resolution picks correct one
        Test::new(
            r#"module Test
            protocol Factory {
                init()
                init(value: Int)
            }
            func makeDefault[T]() -> T where T: Factory {
                return T()
            }
            func makeWithValue[T](v: Int) -> T where T: Factory {
                return T(value: v)
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod basic_static_method {
    use super::*;

    #[test]
    fn static_method_on_type_parameter() {
        // Basic case: T.create() where T: Protocol with static method
        Test::new(
            r#"module Test
            protocol Factory {
                static func create() -> Self
            }
            func make[T]() -> T where T: Factory {
                return T.create()
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn static_method_with_arguments() {
        // T.create(value: 1) - needs explicit external label syntax
        Test::new(
            r#"module Test
            protocol Factory {
                static func create(value value: Int) -> Self
            }
            func make[T](v: Int) -> T where T: Factory {
                return T.create(value: v)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn static_method_non_self_return() {
        // Static method returning something other than Self
        Test::new(
            r#"module Test
            protocol Describable {
                static func typeName() -> String
            }
            func getName[T]() -> String where T: Describable {
                return T.typeName()
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn static_method_with_self_parameter() {
        // Static method with Self in parameter - needs explicit external label syntax
        Test::new(
            r#"module Test
            protocol Factory {
                static func combine(a a: Self, b b: Self) -> Self
            }
            func merged[T](x: T, y: T) -> T where T: Factory {
                return T.combine(a: x, b: y)
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod self_substitution {
    use super::*;

    #[test]
    fn init_return_type_is_type_parameter() {
        // T() returns T, not Self
        Test::new(
            r#"module Test
            protocol Factory {
                init()
            }
            func make[T]() -> T where T: Factory {
                let result: T = T();
                return result
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn static_method_return_type_is_type_parameter() {
        // T.create() returns T, not Self
        Test::new(
            r#"module Test
            protocol Factory {
                static func create() -> Self
            }
            func make[T]() -> T where T: Factory {
                let result: T = T.create();
                return result
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn chained_static_calls() {
        // T.create().clone() - chaining calls
        Test::new(
            r#"module Test
            protocol Factory {
                static func create() -> Self
                func clone() -> Self
            }
            func makeAndClone[T]() -> T where T: Factory {
                return T.create().clone()
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod multiple_bounds {
    use super::*;

    #[test]
    fn init_from_one_of_multiple_bounds() {
        // Init found in one protocol, other method in another
        Test::new(
            r#"module Test
            protocol Creatable {
                init()
            }
            protocol Describable {
                func describe() -> String
            }
            func make[T]() -> T where T: Creatable, T: Describable {
                return T()
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn static_method_from_one_of_multiple_bounds() {
        // Static method from one protocol, use method from another
        Test::new(
            r#"module Test
            protocol Factory {
                static func create() -> Self
            }
            protocol Describable {
                func describe() -> String
            }
            func makeAndDescribe[T]() -> String where T: Factory, T: Describable {
                let item: T = T.create();
                return item.describe()
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn using_and_keyword_for_bounds() {
        // T: Factory and Describable syntax
        Test::new(
            r#"module Test
            protocol Factory {
                static func create() -> Self
            }
            protocol Describable {
                func describe() -> String
            }
            func makeAndDescribe[T]() -> String where T: Factory and Describable {
                let item: T = T.create();
                return item.describe()
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod error_cases {
    use super::*;

    #[test]
    fn no_init_in_bounds() {
        // T() but protocol has no init
        Test::new(
            r#"module Test
            protocol Empty {
                func doSomething() -> Int
            }
            func make[T]() -> T where T: Empty {
                return T()
            }
        "#,
        )
        .expect(HasError("init"));
    }

    #[test]
    fn no_static_method_in_bounds() {
        // T.create() but protocol has no such static method
        Test::new(
            r#"module Test
            protocol Factory {
                func instanceMethod() -> Int
            }
            func make[T]() -> T where T: Factory {
                return T.create()
            }
        "#,
        )
        .expect(HasError("create"));
    }

    #[test]
    fn unconstrained_type_param_init() {
        // T() with no bounds at all
        Test::new(
            r#"module Test
            func make[T]() -> T {
                return T()
            }
        "#,
        )
        .expect(HasError(""));
    }

    #[test]
    fn unconstrained_type_param_static_method() {
        // T.create() with no bounds at all
        Test::new(
            r#"module Test
            func make[T]() -> T {
                return T.create()
            }
        "#,
        )
        .expect(HasError(""));
    }

    #[test]
    fn standalone_type_parameter_error() {
        // let x = T should be an error
        Test::new(
            r#"module Test
            protocol Factory {
                init()
            }
            func bad[T]() where T: Factory {
                let x = T;
            }
        "#,
        )
        .expect(HasError(""));
    }

    #[test]
    fn ambiguous_init() {
        // Same init signature in multiple bounds
        Test::new(
            r#"module Test
            protocol Factory1 {
                init()
            }
            protocol Factory2 {
                init()
            }
            func make[T]() -> T where T: Factory1, T: Factory2 {
                return T()
            }
        "#,
        )
        .expect(HasError("ambiguous"));
    }

    #[test]
    fn ambiguous_static_method() {
        // Same static method signature in multiple bounds
        Test::new(
            r#"module Test
            protocol Factory1 {
                static func create() -> Self
            }
            protocol Factory2 {
                static func create() -> Self
            }
            func make[T]() -> T where T: Factory1, T: Factory2 {
                return T.create()
            }
        "#,
        )
        .expect(HasError("ambiguous"));
    }

    #[test]
    fn wrong_argument_labels() {
        // T(wrong: 1) when protocol expects T(value: 1)
        Test::new(
            r#"module Test
            protocol Factory {
                init(value: Int)
            }
            func make[T]() -> T where T: Factory {
                return T(wrong: 1)
            }
        "#,
        )
        .expect(HasError(""));
    }

    #[test]
    fn wrong_argument_count() {
        // T() when protocol expects T(value: Int)
        Test::new(
            r#"module Test
            protocol Factory {
                init(value: Int)
            }
            func make[T]() -> T where T: Factory {
                return T()
            }
        "#,
        )
        .expect(HasError(""));
    }

    #[test]
    fn calling_instance_method_as_static() {
        // T.instanceMethod() should error - it's not static
        Test::new(
            r#"module Test
            protocol Factory {
                func instanceMethod() -> Self
            }
            func make[T]() -> T where T: Factory {
                return T.instanceMethod()
            }
        "#,
        )
        .expect(HasError(""));
    }
}

mod nested_scopes {
    use super::*;

    #[test]
    fn inner_function_uses_outer_type_param() {
        // Nested function accessing outer's type parameter
        Test::new(
            r#"module Test
            protocol Factory {
                init()
            }
            func outer[T]() -> T where T: Factory {
                func inner() -> T {
                    return T()
                }
                return inner()
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn struct_method_uses_struct_type_param() {
        // Method in generic struct using the struct's type parameter
        Test::new(
            r#"module Test
            protocol Factory {
                init()
            }
            struct Container[T] where T: Factory {
                func makeOne() -> T {
                    return T()
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn struct_method_uses_struct_type_param_static() {
        // Method using static method on struct's type parameter
        Test::new(
            r#"module Test
            protocol Factory {
                static func create() -> Self
            }
            struct Container[T] where T: Factory {
                func makeOne() -> T {
                    return T.create()
                }
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod inherited_protocols {
    use super::*;

    #[test]
    fn init_from_inherited_protocol() {
        // Init declared in parent protocol, bound is child protocol
        Test::new(
            r#"module Test
            protocol Base {
                init()
            }
            protocol Child: Base {
                func extra() -> Int
            }
            func make[T]() -> T where T: Child {
                return T()
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn static_method_from_inherited_protocol() {
        // Static method in parent, bound is child
        Test::new(
            r#"module Test
            protocol Base {
                static func create() -> Self
            }
            protocol Child: Base {
                func extra() -> Int
            }
            func make[T]() -> T where T: Child {
                return T.create()
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn diamond_inheritance_init() {
        // Init from common ancestor, accessed through diamond
        Test::new(
            r#"module Test
            protocol Base {
                init()
            }
            protocol Left: Base {}
            protocol Right: Base {}
            func make[T]() -> T where T: Left, T: Right {
                return T()
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod edge_cases {
    use super::*;

    #[test]
    fn generic_protocol_bound() {
        // T: Container[E] with generic protocol
        Test::new(
            r#"module Test
            protocol Container[E] {
                static func empty() -> Self
            }
            func makeEmpty[T, E]() -> T where T: Container[E] {
                return T.empty()
            }
        "#,
        )
        .expect(HasError("generic protocol bounds"));
    }

    #[test]
    fn multiple_type_params_different_bounds() {
        // Two type params, each with their own init
        Test::new(
            r#"module Test
            protocol FactoryA {
                init()
            }
            protocol FactoryB {
                init()
            }
            func makeBoth[A, B]() -> (A, B) where A: FactoryA, B: FactoryB {
                return (A(), B())
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn type_param_in_static_method_argument() {
        // T.combine(a, b) where arguments are also of type T - needs explicit external labels
        Test::new(
            r#"module Test
            protocol Combinable {
                static func combine(left left: Self, right right: Self) -> Self
            }
            func merged[T](a: T, b: T) -> T where T: Combinable {
                return T.combine(left: a, right: b)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn static_method_returning_different_type() {
        // Static method not returning Self
        Test::new(
            r#"module Test
            protocol Counter {
                static func count() -> Int
            }
            func getCount[T]() -> Int where T: Counter {
                return T.count()
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn init_and_static_in_same_function() {
        // Using both T() and T.method() in same function
        Test::new(
            r#"module Test
            protocol Factory {
                init()
                static func create() -> Self
            }
            func makeBothWays[T]() -> (T, T) where T: Factory {
                return (T(), T.create())
            }
        "#,
        )
        .expect(Compiles);
    }
}
