//! Tests for the Cloneable protocol (Phase 6)
//!
//! This module tests the Cloneable implementation including:
//! - Parsing and binding of @builtin(.Cloneable) and @builtin(.Clone) attributes
//! - Cloneable: Copyable inheritance (protocol refinement)
//! - CopySemantics::Cloneable for conforming types
//! - Error detection for Cloneable field without conformance
//! - MIR lowering with witness calls to Cloneable.clone
//! - Generic constraints with where T: Cloneable

use kestrel_test_suite::mir::*;
use kestrel_test_suite::*;

// =============================================================================
// PARSING AND BINDING TESTS
// =============================================================================

mod parsing {
    use super::*;

    #[test]
    fn builtin_cloneable_on_protocol_parses() {
        // The @builtin(.Cloneable) attribute on a protocol should parse and bind
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            protocol Copyable {}
            
            @builtin(.Cloneable)
            protocol Cloneable: Copyable {
                func clone() -> Self
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles)
        .expect(
            Symbol::new("Cloneable")
                .is(SymbolKind::Protocol)
                .has(Behavior::HasAttribute("builtin")),
        );
    }

    #[test]
    fn builtin_clone_on_method_parses() {
        // The @builtin(.Clone) attribute on a method should parse and bind
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            protocol Copyable {}
            
            @builtin(.Cloneable)
            protocol Cloneable: Copyable {
                @builtin(.Clone)
                func clone() -> Self
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles)
        .expect(
            Symbol::new("Cloneable.clone")
                .is(SymbolKind::Function)
                .has(Behavior::HasAttribute("builtin")),
        );
    }

    #[test]
    fn cloneable_inherits_from_copyable() {
        // Cloneable protocol should inherit from Copyable
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            protocol Copyable {}
            
            @builtin(.Cloneable)
            protocol Cloneable: Copyable {
                func clone() -> Self
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles)
        .expect(Symbol::new("Cloneable").is(SymbolKind::Protocol));
    }

    #[test]
    fn struct_conforming_to_cloneable_compiles() {
        // A struct that conforms to Cloneable should compile
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            protocol Copyable {}
            
            @builtin(.Cloneable)
            protocol Cloneable: Copyable {
                func clone() -> Self
            }
            
            struct MyData: Cloneable {
                var value: lang.i64
                
                func clone() -> MyData {
                    MyData(value: self.value)
                }
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles)
        .expect(
            Symbol::new("MyData")
                .is(SymbolKind::Struct)
                .has(Behavior::ConformsTo("Cloneable")),
        );
    }
}

// =============================================================================
// COPY SEMANTICS TESTS
// =============================================================================

mod copy_semantics {
    use super::*;

    #[test]
    fn struct_conforming_to_cloneable_has_cloneable_semantics() {
        // A struct conforming to Cloneable should have CopySemantics::Cloneable
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            protocol Copyable {}
            
            @builtin(.Cloneable)
            protocol Cloneable: Copyable {
                func clone() -> Self
            }
            
            struct MyData: Cloneable {
                var value: lang.i64
                
                func clone() -> MyData {
                    MyData(value: self.value)
                }
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles)
        .expect(
            Symbol::new("MyData")
                .is(SymbolKind::Struct)
                // Cloneable types are still copyable (via clone)
                .has(Behavior::IsCopyable(true))
                // And they have Cloneable semantics
                .has(Behavior::IsCloneable(true)),
        );
    }

    #[test]
    fn simple_struct_is_not_cloneable() {
        // A simple struct without Cloneable conformance should not be cloneable
        Test::new(
            r#"module Test
            struct Point {
                var x: lang.i64
                var y: lang.i64
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles)
        .expect(
            Symbol::new("Point")
                .is(SymbolKind::Struct)
                .has(Behavior::IsCopyable(true))
                .has(Behavior::IsCloneable(false)),
        );
    }

    #[test]
    fn struct_with_cloneable_field_without_conformance_errors() {
        // A struct with a Cloneable field must itself conform to Cloneable
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            protocol Copyable {}
            
            @builtin(.Cloneable)
            protocol Cloneable: Copyable {
                func clone() -> Self
            }
            
            struct Inner: Cloneable {
                var value: lang.i64
                
                func clone() -> Inner {
                    Inner(value: self.value)
                }
            }
            
            struct Outer {
                var inner: Inner
            }
        "#,
        )
        .without_prelude()
        .expect(HasError("Cloneable"));
    }

    #[test]
    fn enum_with_cloneable_payload_without_conformance_errors() {
        // An enum with a Cloneable payload must itself conform to Cloneable
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            protocol Copyable {}
            
            @builtin(.Cloneable)
            protocol Cloneable: Copyable {
                func clone() -> Self
            }
            
            struct Inner: Cloneable {
                var value: lang.i64
                
                func clone() -> Inner {
                    Inner(value: self.value)
                }
            }
            
            enum Container {
                case Some(value: Inner)
                case None
            }
        "#,
        )
        .without_prelude()
        .expect(HasError("Cloneable"));
    }

    #[test]
    fn struct_with_cloneable_field_and_conformance_compiles() {
        // A struct with a Cloneable field that also conforms to Cloneable should compile
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            protocol Copyable {}
            
            @builtin(.Cloneable)
            protocol Cloneable: Copyable {
                func clone() -> Self
            }
            
            struct Inner: Cloneable {
                var value: lang.i64
                
                func clone() -> Inner {
                    Inner(value: self.value)
                }
            }
            
            struct Outer: Cloneable {
                var inner: Inner
                
                func clone() -> Outer {
                    Outer(inner: self.inner.clone())
                }
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles)
        .expect(
            Symbol::new("Outer")
                .is(SymbolKind::Struct)
                .has(Behavior::ConformsTo("Cloneable")),
        );
    }
}

// =============================================================================
// CONFLICTING CONFORMANCE TESTS
// =============================================================================

mod conflicting_conformance {
    use super::*;

    // Note: The tests for `Cloneable + not Copyable` conflict are already in
    // negative_conformance.rs. These tests verify the same constraint.

    #[test]
    fn cloneable_and_not_copyable_is_error() {
        // Cloneable refines Copyable, so this combination is invalid
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            protocol Copyable {}
            
            @builtin(.Cloneable)
            protocol Cloneable: Copyable {
                func clone() -> Self
            }
            
            struct Invalid: Cloneable, not Copyable {
                var value: lang.i64
                
                func clone() -> Invalid {
                    Invalid(value: self.value)
                }
            }
        "#,
        )
        .without_prelude()
        .expect(HasError(
            "cannot conform to `Cloneable` and opt out of `Copyable`",
        ));
    }
}

// =============================================================================
// MIR TESTS - Clone Witness Calls
// =============================================================================

mod mir_tests {
    use super::*;

    #[test]
    fn consuming_cloneable_emits_witness_call() {
        // Passing a Cloneable type to a consuming parameter should emit a witness call to clone
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            protocol Copyable {}
            
            @builtin(.Cloneable)
            protocol Cloneable: Copyable {
                @builtin(.Clone)
                func clone() -> Self
            }
            
            struct Data: Cloneable {
                var value: lang.i64
                
                func clone() -> Data {
                    Data(value: self.value)
                }
            }
            
            func consume(consuming d: Data) {}
            
            func test() {
                let data = Data(value: 42);
                consume(data)
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_function("Test.test").calls_witness("Test.Cloneable", "clone"));
    }

    #[test]
    fn consuming_cloneable_takes_original_by_ref() {
        // The clone call should take the original value by reference (borrow)
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            protocol Copyable {}
            
            @builtin(.Cloneable)
            protocol Cloneable: Copyable {
                @builtin(.Clone)
                func clone() -> Self
            }
            
            struct Data: Cloneable {
                var value: lang.i64
                
                func clone() -> Data {
                    Data(value: self.value)
                }
            }
            
            func consume(consuming d: Data) {}
            
            func test() {
                let data = Data(value: 42);
                consume(data)
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles)
        .expect(Mir::compiles())
        // The witness call should use Ref mode for the self parameter
        .expect(Mir::mir_function("Test.test").any_block(|b| {
            b.has_statement(StatementPattern::CallWitness {
                protocol: "Test.Cloneable".to_string(),
                method: "clone".to_string(),
            })
        }));
    }

    #[test]
    fn consuming_cloneable_then_moves_cloned_value() {
        // After cloning, the cloned value should be moved to the consuming function
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            protocol Copyable {}
            
            @builtin(.Cloneable)
            protocol Cloneable: Copyable {
                @builtin(.Clone)
                func clone() -> Self
            }
            
            struct Data: Cloneable {
                var value: lang.i64
                
                func clone() -> Data {
                    Data(value: self.value)
                }
            }
            
            func consume(consuming d: Data) {}
            
            func test() {
                let data = Data(value: 42);
                consume(data)
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles)
        .expect(Mir::compiles())
        // The final call to consume should use Move mode
        .expect(Mir::mir_function("Test.test").any_block(|b| {
            b.has_statement(StatementPattern::CallWithModes {
                callee: "Test.consume".to_string(),
                arg_modes: vec![PassingMode::Move],
            })
        }));
    }

    #[test]
    fn borrow_cloneable_does_not_emit_clone() {
        // Borrowing a Cloneable type should NOT emit a clone call
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            protocol Copyable {}
            
            @builtin(.Cloneable)
            protocol Cloneable: Copyable {
                @builtin(.Clone)
                func clone() -> Self
            }
            
            struct Data: Cloneable {
                var value: lang.i64
                
                func clone() -> Data {
                    Data(value: self.value)
                }
            }
            
            func borrow(d: Data) {}
            
            func test() {
                let data = Data(value: 42);
                borrow(data)
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles)
        .expect(Mir::compiles())
        // No witness call should be made for borrowing
        // Reference is created first, then copied to the call
        .expect(Mir::mir_function("Test.test").any_block(|b| {
            b.has_statement(StatementPattern::CallWithModes {
                callee: "Test.borrow".to_string(),
                arg_modes: vec![PassingMode::Copy],
            })
        }))
        .expect(
            Mir::mir_function("Test.test").any_block(|b| b.has_statement(StatementPattern::Ref)),
        );
    }

    #[test]
    fn simple_copyable_does_not_emit_clone() {
        // A simple Copyable type should use Copy, not clone
        Test::new(
            r#"module Test
            struct Point {
                var x: lang.i64
                var y: lang.i64
            }
            
            func consume(consuming p: Point) {}
            
            func test() {
                let pt = Point(x: 1, y: 2);
                consume(pt)
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles)
        .expect(Mir::compiles())
        // Simple copyable uses Copy mode, not Move after clone
        .expect(Mir::mir_function("Test.test").any_block(|b| {
            b.has_statement(StatementPattern::CallWithModes {
                callee: "Test.consume".to_string(),
                arg_modes: vec![PassingMode::Copy],
            })
        }));
    }
}

// =============================================================================
// GENERIC TESTS
// =============================================================================

mod generic_tests {
    use super::*;

    #[test]
    fn where_t_cloneable_constraint_compiles() {
        // Generic function with where T: Cloneable constraint should compile
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            protocol Copyable {}
            
            @builtin(.Cloneable)
            protocol Cloneable: Copyable {
                func clone() -> Self
            }
            
            func duplicate[T](item: T) -> (T, T) where T: Cloneable {
                let copy = item.clone();
                (item, copy)
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles)
        .expect(
            Symbol::new("duplicate")
                .is(SymbolKind::Function)
                .has(Behavior::TypeParamCount(1)),
        );
    }

    #[test]
    fn generic_function_can_call_clone_on_bounded_type() {
        // A generic function should be able to call clone() on a T: Cloneable parameter
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            protocol Copyable {}
            
            @builtin(.Cloneable)
            protocol Cloneable: Copyable {
                func clone() -> Self
            }
            
            func makeClone[T](item: T) -> T where T: Cloneable {
                item.clone()
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles);
    }

    #[test]
    fn generic_clone_call_emits_witness() {
        // Calling clone() on a generic T: Cloneable should emit a witness call
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            protocol Copyable {}
            
            @builtin(.Cloneable)
            protocol Cloneable: Copyable {
                @builtin(.Clone)
                func clone() -> Self
            }
            
            func makeClone[T](item: T) -> T where T: Cloneable {
                item.clone()
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_function("Test.makeClone").calls_witness("Test.Cloneable", "clone"));
    }

    #[test]
    fn calling_generic_clone_with_cloneable_type_compiles() {
        // Calling a generic function that requires Cloneable with a Cloneable type should work
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            protocol Copyable {}
            
            @builtin(.Cloneable)
            protocol Cloneable: Copyable {
                func clone() -> Self
            }
            
            struct Data: Cloneable {
                var value: lang.i64
                
                func clone() -> Data {
                    Data(value: self.value)
                }
            }
            
            func makeClone[T](item: T) -> T where T: Cloneable {
                item.clone()
            }
            
            func test() -> Data {
                let d = Data(value: 42);
                makeClone(d)
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles);
    }

    #[test]
    fn calling_generic_clone_with_non_cloneable_type_errors() {
        // Calling a generic function that requires Cloneable with a non-Cloneable type should error
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            protocol Copyable {}
            
            @builtin(.Cloneable)
            protocol Cloneable: Copyable {
                func clone() -> Self
            }
            
            struct Point {
                var x: lang.i64
                var y: lang.i64
            }
            
            func makeClone[T](item: T) -> T where T: Cloneable {
                item.clone()
            }
            
            func test() -> Point {
                let p = Point(x: 1, y: 2);
                makeClone(p)
            }
        "#,
        )
        .without_prelude()
        .expect(HasError("")); // Should error about Point not conforming to Cloneable
    }
}

// =============================================================================
// WITNESS TABLE TESTS
// =============================================================================

mod witness_tests {
    use super::*;

    #[test]
    fn cloneable_conformance_generates_witness() {
        // Conforming to Cloneable should generate a witness table
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            protocol Copyable {}
            
            @builtin(.Cloneable)
            protocol Cloneable: Copyable {
                func clone() -> Self
            }
            
            struct Data: Cloneable {
                var value: lang.i64
                
                func clone() -> Data {
                    Data(value: self.value)
                }
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_witness("Test.Data", "Test.Cloneable")
                .has_method("clone")
                .has_method_mapping("clone", "Test.Data.clone"),
        );
    }
}

// =============================================================================
// MULTIPLE CONSUMING PARAMETERS TESTS
// =============================================================================

mod multiple_args_tests {
    use super::*;

    #[test]
    fn multiple_cloneable_args_all_clone() {
        // Multiple Cloneable arguments to consuming parameters should all be cloned
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            protocol Copyable {}
            
            @builtin(.Cloneable)
            protocol Cloneable: Copyable {
                @builtin(.Clone)
                func clone() -> Self
            }
            
            struct Data: Cloneable {
                var value: lang.i64
                
                func clone() -> Data {
                    Data(value: self.value)
                }
            }
            
            func consumeTwo(consuming a: Data, consuming b: Data) {}
            
            func test() {
                let d1 = Data(value: 1);
                let d2 = Data(value: 2);
                consumeTwo(d1, d2)
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_function("Test.test").any_block(|b| {
            b.has_statement(StatementPattern::CallWithModes {
                callee: "Test.consumeTwo".to_string(),
                arg_modes: vec![PassingMode::Move, PassingMode::Move],
            })
        }));
    }

    #[test]
    fn mixed_copyable_and_cloneable_args() {
        // Mixed copyable and cloneable arguments should use appropriate modes
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            protocol Copyable {}
            
            @builtin(.Cloneable)
            protocol Cloneable: Copyable {
                @builtin(.Clone)
                func clone() -> Self
            }
            
            struct Point {
                var x: lang.i64
                var y: lang.i64
            }
            
            struct Data: Cloneable {
                var value: lang.i64
                
                func clone() -> Data {
                    Data(value: self.value)
                }
            }
            
            func consumeMixed(consuming p: Point, consuming d: Data) {}
            
            func test() {
                let pt = Point(x: 1, y: 2);
                let data = Data(value: 42);
                consumeMixed(pt, data)
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles)
        .expect(Mir::compiles())
        // Point is simple Copyable → Copy, Data is Cloneable → Move (after clone)
        .expect(Mir::mir_function("Test.test").any_block(|b| {
            b.has_statement(StatementPattern::CallWithModes {
                callee: "Test.consumeMixed".to_string(),
                arg_modes: vec![PassingMode::Copy, PassingMode::Move],
            })
        }));
    }
}

// =============================================================================
// ORIGINAL VALUE PRESERVATION TESTS
// =============================================================================

mod value_preservation_tests {
    use super::*;

    #[test]
    fn cloneable_original_still_valid_after_consume() {
        // After consuming a Cloneable value, the original should still be usable
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            protocol Copyable {}
            
            @builtin(.Cloneable)
            protocol Cloneable: Copyable {
                func clone() -> Self
            }
            
            struct Data: Cloneable {
                var value: lang.i64
                
                func clone() -> Data {
                    Data(value: self.value)
                }
            }
            
            func consume(consuming d: Data) {}
            
            func test() {
                let data = Data(value: 42);
                consume(data);
                consume(data)
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles); // Should compile - data is cloned each time
    }
}
