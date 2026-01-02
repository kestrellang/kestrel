//! Builtin protocol tests
//!
//! Tests for the `@builtin(.Feature)` attribute on protocols.

use kestrel_test_suite::*;

// =============================================================================
// SUCCESS CASES
// =============================================================================

mod success {
    use super::*;

    #[test]
    fn builtin_copyable_on_marker_protocol() {
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            protocol Copyable {}
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Copyable")
                .is(SymbolKind::Protocol)
                .has(Behavior::HasAttribute("builtin")),
        );
    }

    #[test]
    fn builtin_expressible_by_int_literal_protocol() {
        Test::new(
            r#"module Test
            @builtin(.ExpressibleByIntLiteral)
            protocol ExpressibleByIntLiteral {
                init(intLiteral value: Int)
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("ExpressibleByIntLiteral")
                .is(SymbolKind::Protocol)
                .has(Behavior::HasAttribute("builtin")),
        );
    }

    #[test]
    fn multiple_builtin_protocols() {
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            protocol Copyable {}
            
            @builtin(.ExpressibleByIntLiteral)
            protocol ExpressibleByIntLiteral {
                init(intLiteral value: Int)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn builtin_with_public_visibility() {
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            public protocol Copyable {}
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Copyable")
                .is(SymbolKind::Protocol)
                .has(Behavior::Visibility(Visibility::Public))
                .has(Behavior::HasAttribute("builtin")),
        );
    }
}

// =============================================================================
// ERROR: MISSING ARGUMENT
// =============================================================================

mod missing_argument {
    use super::*;

    #[test]
    fn builtin_without_argument() {
        Test::new(
            r#"module Test
            @builtin
            protocol Foo {}
        "#,
        )
        .expect(HasError("@builtin requires a language feature argument"));
    }

    #[test]
    fn builtin_with_empty_parens() {
        Test::new(
            r#"module Test
            @builtin()
            protocol Foo {}
        "#,
        )
        .expect(HasError("@builtin requires a language feature argument"));
    }
}

// =============================================================================
// ERROR: UNKNOWN FEATURE
// =============================================================================

mod unknown_feature {
    use super::*;

    #[test]
    fn unknown_language_feature() {
        Test::new(
            r#"module Test
            @builtin(.UnknownFeature)
            protocol Foo {}
        "#,
        )
        .expect(HasError("unknown language feature"));
    }

    #[test]
    fn misspelled_language_feature() {
        Test::new(
            r#"module Test
            @builtin(.Copable)
            protocol Foo {}
        "#,
        )
        .expect(HasError("unknown language feature"));
    }
}

// =============================================================================
// ERROR: WRONG SYMBOL KIND
// =============================================================================

mod wrong_symbol_kind {
    use super::*;

    #[test]
    fn builtin_protocol_on_struct() {
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            struct Foo {}
        "#,
        )
        .expect(HasError(
            "@builtin(.Copyable) can only be applied to a protocol",
        ));
    }

    #[test]
    fn builtin_protocol_on_enum() {
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            enum Color { case Red }
        "#,
        )
        .expect(HasError(
            "@builtin(.Copyable) can only be applied to a protocol",
        ));
    }

    #[test]
    fn builtin_protocol_on_function() {
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            func foo() {}
        "#,
        )
        .expect(HasError(
            "@builtin(.Copyable) can only be applied to a protocol",
        ));
    }
}

// =============================================================================
// ERROR: NON-MARKER PROTOCOL FOR MARKER-REQUIRED BUILTIN
// =============================================================================

mod non_marker_protocol {
    use super::*;

    #[test]
    fn copyable_on_protocol_with_method() {
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            protocol Copyable {
                func copy() -> Self
            }
        "#,
        )
        .expect(HasError("must be a marker protocol"));
    }

    #[test]
    fn copyable_on_protocol_with_associated_type() {
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            protocol Copyable {
                type Element;
            }
        "#,
        )
        .expect(HasError("must be a marker protocol"));
    }
}

// =============================================================================
// ERROR: DUPLICATE BUILTIN
// =============================================================================

mod duplicate_builtin {
    use super::*;

    #[test]
    fn duplicate_copyable_builtin() {
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            protocol Copyable1 {}
            
            @builtin(.Copyable)
            protocol Copyable2 {}
        "#,
        )
        .expect(HasError("already defined"));
    }
}

// =============================================================================
// ERROR: INVALID ARGUMENT FORMAT
// =============================================================================

mod invalid_argument_format {
    use super::*;

    #[test]
    fn builtin_with_string_argument() {
        Test::new(
            r#"module Test
            @builtin("Copyable")
            protocol Foo {}
        "#,
        )
        .expect(HasError("expected implicit member syntax"));
    }

    #[test]
    fn builtin_with_integer_argument() {
        Test::new(
            r#"module Test
            @builtin(42)
            protocol Foo {}
        "#,
        )
        .expect(HasError("expected implicit member syntax"));
    }

    #[test]
    fn builtin_with_labeled_argument() {
        Test::new(
            r#"module Test
            @builtin(feature: .Copyable)
            protocol Foo {}
        "#,
        )
        .expect(HasError("expected implicit member syntax"));
    }
}
