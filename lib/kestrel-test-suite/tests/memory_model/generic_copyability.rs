//! Tests for generic copyability (Phase 7)
//!
//! This module tests generic type parameters with copyability bounds including:
//! - `where T: not Copyable` syntax for relaxing implicit Copyable bound
//! - Move tracking for type parameters with `not Copyable` bound
//! - Type parameters can be copied by default (implicit Copyable bound)

use kestrel_test_suite::*;

// =============================================================================
// PARSING TESTS - WHERE CLAUSE NEGATIVE BOUNDS
// =============================================================================

mod parsing {
    use super::*;

    #[test]
    fn parses_not_copyable_in_where_clause() {
        // Basic syntax: where T: not Copyable
        Test::new(
            r#"module Test
            import Prelude
            
            func process[T](consuming x: T) where T: not Copyable { }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn parses_mixed_bounds_in_where_clause() {
        // Mix of positive and negative bounds
        Test::new(
            r#"module Test
            import Prelude
            protocol Displayable {}
            
            func process[T, U](consuming x: T, y: U) where T: not Copyable, U: Displayable { }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn parses_negative_bound_on_struct_generic() {
        // Struct with generic type parameter that can accept non-copyable types
        Test::new(
            r#"module Test
            import Prelude
            
            struct Box[T] where T: not Copyable {
                var value: T
            }
        "#,
        )
        .expect(Compiles);
    }
}

// =============================================================================
// SEMANTIC TESTS - TYPE PARAMETER COPYABILITY
// =============================================================================

mod semantic {
    use super::*;

    #[test]
    fn type_parameter_is_copyable_by_default() {
        // Without `not Copyable`, type parameter values can be copied
        Test::new(
            r#"module Test
            func duplicate[T](x: T) -> (T, T) {
                return (x, x)  // T is implicitly Copyable, so this works
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn type_parameter_with_not_copyable_cannot_be_duplicated() {
        // With `not Copyable`, using a value twice should be an error
        Test::new(
            r#"module Test
            import Prelude
            
            func process[T](consuming x: T) where T: not Copyable {
                let a = x;
                let b = x;  // Error: use after move
            }
        "#,
        )
        .expect(HasError("use of moved value"));
    }

    #[test]
    fn type_parameter_with_not_copyable_can_be_moved_once() {
        // With `not Copyable`, moving once is fine
        Test::new(
            r#"module Test
            import Prelude
            
            func accept[T](consuming x: T) where T: not Copyable { }
            
            func forward[T](consuming x: T) where T: not Copyable {
                accept(x);  // x is moved here, that's fine
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn type_parameter_with_not_copyable_use_after_move() {
        // With `not Copyable`, using after move should error
        Test::new(
            r#"module Test
            import Prelude
            
            func accept[T](consuming x: T) where T: not Copyable { }
            
            func forward[T](consuming x: T) where T: not Copyable {
                accept(x);  // x is moved here
                accept(x);  // Error: use after move
            }
        "#,
        )
        .expect(HasError("use of moved value"));
    }
}

// =============================================================================
// FUNCTION CALL TESTS - PASSING TYPE PARAMETERS
// =============================================================================

mod function_calls {
    use super::*;

    #[test]
    fn can_pass_non_copyable_type_to_not_copyable_generic() {
        // A non-copyable struct should be passable to a function with `where T: not Copyable`
        Test::new(
            r#"module Test
            import Prelude
            
            struct Handle: not Copyable {
                var fd: lang.i64
            }
            
            func process[T](consuming x: T) where T: not Copyable { }
            
            func test() {
                var h = Handle(fd: 1);
                process(h);  // This should work
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn can_pass_copyable_type_to_not_copyable_generic() {
        // A copyable type should also work with `where T: not Copyable`
        // (the constraint relaxes the requirement, doesn't mandate non-copyability)
        Test::new(
            r#"module Test
            import Prelude
            
            struct Point {
                var x: lang.i64
                var y: lang.i64
            }
            
            func process[T](consuming x: T) where T: not Copyable { }
            
            func test() {
                var p = Point(x: 1, y: 2);
                process(p);  // Copyable types work too
            }
        "#,
        )
        .expect(Compiles);
    }
}

// =============================================================================
// STRUCT GENERIC TESTS
// =============================================================================

mod struct_generics {
    use super::*;

    #[test]
    fn struct_with_not_copyable_generic_accepts_non_copyable_field() {
        Test::new(
            r#"module Test
            import Prelude
            
            struct Handle: not Copyable {
                var fd: lang.i64
            }
            
            struct Wrapper[T] where T: not Copyable {
                var value: T
            }
            
            func test() {
                var h = Handle(fd: 1);
                var w = Wrapper(value: h);
            }
        "#,
        )
        .expect(Compiles);
    }
}
