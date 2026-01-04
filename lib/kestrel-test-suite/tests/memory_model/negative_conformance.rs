//! Tests for negative conformance syntax (`not Copyable`)
//!
//! Phase 4 of the memory model implementation introduces:
//! - `not Protocol` syntax in conformance lists
//! - Validation that only builtin protocols with implicit conformance can be negated

use kestrel_test_suite::*;

// =============================================================================
// PARSING TESTS
// =============================================================================

mod parsing {
    use super::*;

    #[test]
    fn struct_with_not_copyable() {
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            protocol Copyable {}
            
            struct Handle: not Copyable {}
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn struct_with_protocol_and_not_copyable() {
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            protocol Copyable {}
            
            protocol Resource {}
            
            struct Handle: Resource, not Copyable {}
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn struct_with_not_copyable_and_protocol() {
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            protocol Copyable {}
            
            protocol Resource {}
            
            struct Handle: not Copyable, Resource {}
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn enum_with_not_copyable() {
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            protocol Copyable {}
            
            enum State: not Copyable {
                case Active
                case Inactive
            }
        "#,
        )
        .expect(Compiles);
    }
}

// =============================================================================
// VALIDATION TESTS - ERRORS
// =============================================================================

mod validation_errors {
    use super::*;

    #[test]
    fn not_with_non_builtin_protocol() {
        Test::new(
            r#"module Test
            protocol MyProtocol {}
            
            struct Foo: not MyProtocol {}
        "#,
        )
        .expect(HasError("not a language feature protocol"));
    }

    #[test]
    fn not_with_regular_protocol_that_has_methods() {
        Test::new(
            r#"module Test
            protocol Drawable {
                func draw()
            }
            
            struct Shape: not Drawable {}
        "#,
        )
        .expect(HasError("not a language feature protocol"));
    }

    #[test]
    fn not_with_builtin_that_has_no_implicit_conformance() {
        // ExpressibleByIntLiteral doesn't have implicit conformance, so it can't be negated
        Test::new(
            r#"module Test
            @builtin(.ExpressibleByIntLiteral)
            protocol ExpressibleByIntLiteral {
                init(intLiteral value: Int)
            }
            
            struct Foo: not ExpressibleByIntLiteral {}
        "#,
        )
        .expect(HasError("not a language feature protocol"));
    }

    #[test]
    fn cloneable_and_not_copyable_is_conflicting() {
        // Cloneable refines Copyable, so a type cannot conform to Cloneable while opting out of Copyable
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            protocol Copyable {}
            
            protocol Cloneable: Copyable {
                func clone() -> Self
            }
            
            struct Handle: Cloneable, not Copyable {
                var fd: Int
                
                func clone() -> Handle {
                    Handle(fd: self.fd)
                }
            }
        "#,
        )
        .expect(HasError(
            "cannot conform to `Cloneable` and opt out of `Copyable`",
        ));
    }

    #[test]
    fn cloneable_and_not_copyable_reversed_order() {
        // Same as above but with not Copyable first in the list
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            protocol Copyable {}
            
            protocol Cloneable: Copyable {
                func clone() -> Self
            }
            
            struct Handle: not Copyable, Cloneable {
                var fd: Int
                
                func clone() -> Handle {
                    Handle(fd: self.fd)
                }
            }
        "#,
        )
        .expect(HasError(
            "cannot conform to `Cloneable` and opt out of `Copyable`",
        ));
    }

    #[test]
    fn enum_cloneable_and_not_copyable_is_conflicting() {
        // Same for enums
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            protocol Copyable {}
            
            protocol Cloneable: Copyable {
                func clone() -> Self
            }
            
            enum State: Cloneable, not Copyable {
                case Active
                case Inactive
                
                func clone() -> State {
                    self
                }
            }
        "#,
        )
        .expect(HasError(
            "cannot conform to `Cloneable` and opt out of `Copyable`",
        ));
    }
}

// =============================================================================
// SEMANTIC TESTS - BEHAVIOR
// =============================================================================

mod semantic {
    use super::*;

    #[test]
    fn negative_conformance_tracked_in_behavior() {
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            protocol Copyable {}
            
            struct Handle: not Copyable {
                var fd: Int
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Handle")
                .is(SymbolKind::Struct)
                .has(Behavior::HasNegativeConformance("Copyable")),
        );
    }

    #[test]
    fn positive_conformance_not_affected() {
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            protocol Copyable {}
            
            protocol Resource {}
            
            struct Handle: Resource, not Copyable {
                var fd: Int
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Handle")
                .is(SymbolKind::Struct)
                .has(Behavior::ConformsTo("Resource"))
                .has(Behavior::HasNegativeConformance("Copyable")),
        );
    }
}
