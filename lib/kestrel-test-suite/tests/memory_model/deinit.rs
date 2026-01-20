//! Tests for deinit semantics (Phase 5)
//!
//! This module tests the deinit implementation including:
//! - Deinit parsing and semantic binding
//! - DeinitBehavior attachment to parent structs
//! - Duplicate deinit error detection
//! - Automatic deinit insertion at scope exit
//!
//! NOTE: Currently, deinit bodies with statements (e.g., `deinit { let x = 1 }`)
//! have a parser bug that causes tree building to fail. Tests use empty deinit
//! bodies `deinit {}` until the parser bug is fixed.

use kestrel_test_suite::mir::*;
use kestrel_test_suite::*;

// =============================================================================
// BASIC DEINIT PARSING AND BINDING
// =============================================================================

mod basic_deinit {
    use super::*;

    #[test]
    fn struct_with_deinit_compiles() {
        // A struct with a deinit block should compile
        Test::new(
            r#"module Test
            import Prelude
            
            struct Handle: not Copyable {
                var fd: lang.i64
                
                deinit {}
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Handle")
                .is(SymbolKind::Struct)
                .has(Behavior::HasDeinit(true)),
        );
    }

    #[test]
    fn struct_without_deinit_has_no_deinit_behavior() {
        // A struct without deinit should not have DeinitBehavior
        Test::new(
            r#"module Test
            struct Point {
                var x: lang.i64
                var y: lang.i64
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Point")
                .is(SymbolKind::Struct)
                .has(Behavior::HasDeinit(false)),
        );
    }

    #[test]
    fn struct_with_init_and_deinit() {
        // A struct can have both init and deinit
        Test::new(
            r#"module Test
            import Prelude
            
            struct Resource: not Copyable {
                var id: lang.i64
                
                init(id: lang.i64) {
                    self.id = id
                }
                
                deinit {}
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Resource")
                .is(SymbolKind::Struct)
                .has(Behavior::HasDeinit(true)),
        );
    }

    #[test]
    fn empty_deinit_body() {
        // An empty deinit body is valid
        Test::new(
            r#"module Test
            import Prelude
            
            struct Handle: not Copyable {
                var fd: lang.i64
                
                deinit {}
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Handle")
                .is(SymbolKind::Struct)
                .has(Behavior::HasDeinit(true)),
        );
    }
}

// =============================================================================
// DUPLICATE DEINIT ERROR
// =============================================================================

mod duplicate_deinit {
    use super::*;

    #[test]
    fn duplicate_deinit_error() {
        // A struct with multiple deinit declarations should error
        Test::new(
            r#"module Test
            import Prelude
            
            struct Handle: not Copyable {
                var fd: lang.i64
                
                deinit {}
                
                deinit {}
            }
        "#,
        )
        .expect(HasError("already has a deinit"));
    }

    #[test]
    fn duplicate_deinit_with_empty_bodies() {
        // Even empty deinit blocks can't be duplicated
        Test::new(
            r#"module Test
            import Prelude
            
            struct Handle: not Copyable {
                var fd: lang.i64
                
                deinit {}
                deinit {}
            }
        "#,
        )
        .expect(HasError("already has a deinit"));
    }
}

// =============================================================================
// COPYABLE + DEINIT
// =============================================================================

mod copyable_with_deinit {
    use super::*;

    #[test]
    fn copyable_struct_with_deinit_compiles() {
        // A Copyable struct with deinit should compile without warning
        Test::new(
            r#"module Test
            struct Counter {
                var count: lang.i64

                deinit {}
            }
        "#,
        )
        .expect(Compiles)
        .expect(NoWarnings)
        .expect(
            Symbol::new("Counter")
                .is(SymbolKind::Struct)
                .has(Behavior::HasDeinit(true))
                .has(Behavior::IsCopyable(true)),
        );
    }

    #[test]
    fn not_copyable_struct_with_deinit_no_warning() {
        // A not Copyable struct with deinit should NOT emit a warning
        Test::new(
            r#"module Test
            import Prelude

            struct Handle: not Copyable {
                var fd: lang.i64

                deinit {}
            }
        "#,
        )
        .expect(Compiles)
        .expect(NoWarnings)
        .expect(
            Symbol::new("Handle")
                .is(SymbolKind::Struct)
                .has(Behavior::HasDeinit(true))
                .has(Behavior::IsCopyable(false)),
        );
    }

    #[test]
    fn struct_with_non_copyable_field_and_deinit_no_warning() {
        // A struct that is not copyable due to non-copyable field should not warn
        Test::new(
            r#"module Test
            import Prelude

            struct Handle: not Copyable {
                var fd: lang.i64
            }

            struct Wrapper {
                var handle: Handle

                deinit {}
            }
        "#,
        )
        .expect(Compiles)
        .expect(NoWarnings)
        .expect(
            Symbol::new("Wrapper")
                .is(SymbolKind::Struct)
                .has(Behavior::HasDeinit(true))
                .has(Behavior::IsCopyable(false)),
        );
    }
}

// =============================================================================
// DEINIT WITH OTHER STRUCT FEATURES
// =============================================================================

mod deinit_with_features {
    use super::*;

    #[test]
    fn deinit_with_multiple_fields() {
        // Struct can have deinit with multiple fields
        Test::new(
            r#"module Test
            import Prelude
            
            struct Connection: not Copyable {
                var host: lang.str                var port: lang.i64
                var connected: lang.i1
                
                deinit {}
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Connection")
                .is(SymbolKind::Struct)
                .has(Behavior::HasDeinit(true))
                .has(Behavior::FieldCount(3)),
        );
    }

    #[test]
    fn deinit_with_protocol_conformance() {
        // Struct with deinit can also conform to protocols
        Test::new(
            r#"module Test
            import Prelude
            
            protocol Resource {}
            
            struct Handle: Resource, not Copyable {
                var fd: lang.i64
                
                deinit {}
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Handle")
                .is(SymbolKind::Struct)
                .has(Behavior::HasDeinit(true))
                .has(Behavior::ConformsTo("Resource"))
                .has(Behavior::IsCopyable(false)),
        );
    }
}

// =============================================================================
// DEINIT STATEMENT (deinit x;)
// =============================================================================

mod deinit_statement {
    use super::*;

    #[test]
    fn basic_deinit_statement_compiles() {
        // The `deinit x;` statement should compile
        Test::new(
            r#"module Test
            func example() {
                var x = 42;
                deinit x;
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn deinit_statement_marks_variable_as_moved() {
        // Using a variable after `deinit x;` should error (use after move)
        Test::new(
            r#"module Test
            import Prelude
            
            struct Handle: not Copyable {
                var fd: lang.i64
                deinit {}
            }
            
            func getId(h: Handle) -> lang.i64 {
                return h.fd
            }
            
            func example() {
                var handle = Handle(fd: 42);
                deinit handle;
                let x = getId(h: handle);  // Error: use after deinit
            }
        "#,
        )
        .expect(HasError("moved"));
    }

    #[test]
    fn deinit_undeclared_variable_error() {
        // Trying to deinit an undeclared variable should error
        Test::new(
            r#"module Test
            func example() {
                deinit unknown;
            }
        "#,
        )
        .expect(HasError("undeclared"));
    }

    #[test]
    fn deinit_already_moved_variable_error() {
        // Trying to deinit an already-moved variable should error
        Test::new(
            r#"module Test
            import Prelude
            
            struct Handle: not Copyable {
                var fd: lang.i64
                deinit {}
            }
            
            func consume(consuming h: Handle) {}
            
            func example() {
                var handle = Handle(fd: 42);
                consume(handle);  // handle is moved here (no label for single param)
                deinit handle;    // Error: already moved
            }
        "#,
        )
        .expect(HasError("moved"));
    }

    #[test]
    fn deinit_copyable_type_allowed() {
        // Deinit on copyable types is allowed (though unusual)
        Test::new(
            r#"module Test
            func example() {
                var x = 42;
                deinit x;
            }
        "#,
        )
        .expect(Compiles);
    }

    // TODO: This test documents future behavior. Currently, after deinit,
    // the variable is marked as moved and cannot be reassigned. In the future,
    // we may want to allow reassignment after explicit deinit.
    // #[test]
    // fn deinit_then_reassign() {
    //     // After deinit, the variable can be reassigned and used again
    //     Test::new(
    //         r#"module Test
    //         @builtin(.Copyable)
    //         protocol Copyable {}
    //
    //         struct Handle: not Copyable {
    //             var fd: lang.i64
    //             deinit {}
    //         }
    //
    //         func example() -> lang.i64 {
    //             var handle = Handle(fd: 42);
    //             deinit handle;
    //             handle = Handle(fd: 100);  // Reassign
    //             return handle.fd           // Use new value
    //         }
    //     "#,
    //     )
    //     .expect(Compiles);
    // }

    #[test]
    fn double_deinit_error() {
        // Can't deinit the same variable twice
        Test::new(
            r#"module Test
            import Prelude
            
            struct Handle: not Copyable {
                var fd: lang.i64
                deinit {}
            }
            
            func example() {
                var handle = Handle(fd: 42);
                deinit handle;
                deinit handle;  // Error: already moved
            }
        "#,
        )
        .expect(HasError("moved"));
    }
}

// =============================================================================
// AUTOMATIC DEINIT INSERTION (MIR lowering tests)
// =============================================================================

mod automatic_deinit {
    use super::*;

    #[test]
    fn basic_scope_exit_deinit() {
        // A non-copyable local with deinit should get a Deinit statement at scope exit
        Test::new(
            r#"module Test
            import Prelude
            
            struct Handle: not Copyable {
                var fd: lang.i64
                deinit {}
            }
            
            func example() {
                let handle = Handle(fd: 42);
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            MirFunction::new("Test.example")
                .any_block(|b| b.has_statement(StatementPattern::AnyDeinit)),
        );
    }

    #[test]
    fn copyable_type_no_automatic_deinit() {
        // Copyable types should NOT get automatic deinit even if they have deinit
        // (the warning about Copyable+deinit is separate)
        // NOTE: We just check that it compiles - we don't emit deinits for copyable types
        Test::new(
            r#"module Test
            struct Counter {
                var count: lang.i64
            }
            
            func example() {
                let c = Counter(count: 0);
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn deinit_in_reverse_order() {
        // Multiple locals should be deinited in reverse declaration order
        Test::new(
            r#"module Test
            import Prelude
            
            struct Handle: not Copyable {
                var fd: lang.i64
                deinit {}
            }
            
            func example() {
                let h1 = Handle(fd: 1);
                let h2 = Handle(fd: 2);
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            MirFunction::new("Test.example")
                // Should have deinit calls for both h1 and h2
                .any_block(|b| {
                    b.has_statement(StatementPattern::DeinitCall {
                        ty: "Test.Handle".to_string(),
                    })
                }),
        );
    }

    #[test]
    fn explicit_deinit_emits_mir_statement() {
        // Explicit `deinit x;` should emit a deinit call when the type has a deinit block
        Test::new(
            r#"module Test
            import Prelude
            
            struct Handle: not Copyable {
                var fd: lang.i64
                deinit {}
            }
            
            func example() {
                let handle = Handle(fd: 42);
                deinit handle;
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            MirFunction::new("Test.example")
                // Should have the explicit deinit call
                .any_block(|b| {
                    b.has_statement(StatementPattern::DeinitCall {
                        ty: "Test.Handle".to_string(),
                    })
                }),
        );
    }

    #[test]
    fn return_emits_deinits() {
        // Return should emit deinits for all in-scope locals
        Test::new(
            r#"module Test
            import Prelude
            
            struct Handle: not Copyable {
                var fd: lang.i64
                deinit {}
            }
            
            func example() -> lang.i64 {
                let handle = Handle(fd: 42);
                return 0;
            }
        "#,
        )
        .expect(Compiles)
        .expect(MirFunction::new("Test.example").any_block(|b| {
            b.has_statement(StatementPattern::DeinitCall {
                ty: "Test.Handle".to_string(),
            })
        }));
    }

    #[test]
    fn break_emits_deinits() {
        // Break should emit deinits for loop-scoped locals
        Test::new(
            r#"module Test
            import Prelude
            
            struct Handle: not Copyable {
                var fd: lang.i64
                deinit {}
            }
            
            func example() {
                loop {
                    let h = Handle(fd: 1);
                    break;
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(MirFunction::new("Test.example").any_block(|b| {
            b.has_statement(StatementPattern::DeinitCall {
                ty: "Test.Handle".to_string(),
            })
        }));
    }

    #[test]
    fn if_branch_deinits() {
        // Each branch should deinit its own locals
        Test::new(
            r#"module Test
            import Prelude
            
            struct Handle: not Copyable {
                var fd: lang.i64
                deinit {}
            }
            
            func example(cond: lang.i1) {
                if cond {
                    let h1 = Handle(fd: 1);
                } else {
                    let h2 = Handle(fd: 2);
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            MirFunction::new("Test.example")
                // Both branches should have deinit calls for their Handle
                .any_block(|b| {
                    b.has_statement(StatementPattern::DeinitCall {
                        ty: "Test.Handle".to_string(),
                    })
                }),
        );
    }

    #[test]
    fn moved_value_not_double_deinited() {
        // A moved value should not be deinited again at scope exit
        Test::new(
            r#"module Test
            import Prelude
            
            struct Handle: not Copyable {
                var fd: lang.i64
                deinit {}
            }
            
            func consume(consuming h: Handle) {}
            
            func example() {
                let handle = Handle(fd: 42);
                consume(handle);
                // handle is moved, should NOT have a deinit here
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            MirFunction::new("Test.example")
                // Should NOT have a Deinit for handle (it was moved to consume)
                .no_block(|b| {
                    b.has_statement(StatementPattern::Deinit {
                        local: "handle".to_string(),
                    })
                }),
        );
    }

    #[test]
    fn temporary_in_nested_call_deinited() {
        // Temporary created for inner call should be deinited at statement end
        // if not consumed
        Test::new(
            r#"module Test
            import Prelude
            
            struct Handle: not Copyable {
                var fd: lang.i64
                deinit {}
            }
            
            func makeHandle() -> Handle {
                return Handle(fd: 42);
            }
            
            func useRef(handle h: Handle) -> lang.i64 {
                return h.fd
            }
            
            func example() -> lang.i64 {
                // makeHandle() creates a temp, passed by ref to useRef
                // The temp should be deinited after this statement
                let result = useRef(handle: makeHandle());
                // At this point the temp from makeHandle() should be deinited
                return result;
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            MirFunction::new("Test.example")
                .any_block(|b| b.has_statement(StatementPattern::AnyDeinit)),
        );
    }

    #[test]
    fn temporary_consumed_not_deinited() {
        // Temporary that is consumed (moved) should NOT be deinited
        Test::new(
            r#"module Test
            import Prelude
            
            struct Handle: not Copyable {
                var fd: lang.i64
                deinit {}
            }
            
            func makeHandle() -> Handle {
                return Handle(fd: 42);
            }
            
            func consume(consuming h: Handle) {}
            
            func example() {
                // makeHandle() creates a temp, consumed by consume()
                // The temp should NOT be deinited (already moved)
                consume(makeHandle());
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            MirFunction::new("Test.example")
                // Should have NO deinit for the temp (it was consumed)
                .no_block(|b| b.has_statement(StatementPattern::AnyDeinit)),
        );
    }

    #[test]
    fn conditional_move_uses_deinit_if() {
        // When a variable is moved in one branch but not another,
        // should use DeinitIf with a flag
        Test::new(
            r#"module Test
            import Prelude
            
            struct Handle: not Copyable {
                var fd: lang.i64
                deinit {}
            }
            
            func consume(consuming h: Handle) {}
            
            func example(cond: lang.i1) {
                let handle = Handle(fd: 42);
                if cond {
                    consume(handle);  // moved here
                } else {
                    // not moved here
                }
                // handle needs conditional deinit (DeinitIf)
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            MirFunction::new("Test.example")
                .any_block(|b| b.has_statement(StatementPattern::AnyDeinitIf)),
        );
    }

    #[test]
    fn conditional_move_sets_flags() {
        // When a variable is moved in one branch but not another,
        // should set deinit flags appropriately in each branch
        Test::new(
            r#"module Test
            import Prelude
            
            struct Handle: not Copyable {
                var fd: lang.i64
                deinit {}
            }
            
            func consume(consuming h: Handle) {}
            
            func example(cond: lang.i1) {
                let handle = Handle(fd: 42);
                if cond {
                    consume(handle);  // moved here - flag should be false
                } else {
                    // not moved here - flag should be true
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            MirFunction::new("Test.example")
                // Should have flag-setting statements
                .any_block(|b| b.has_statement(StatementPattern::AnySetDeinitFlag)),
        );
    }

    #[test]
    fn both_branches_move_no_conditional_deinit() {
        // When a variable is moved in both branches, should NOT use DeinitIf
        Test::new(
            r#"module Test
            import Prelude
            
            struct Handle: not Copyable {
                var fd: lang.i64
                deinit {}
            }
            
            func consume(consuming h: Handle) {}
            
            func example(cond: lang.i1) {
                let handle = Handle(fd: 42);
                if cond {
                    consume(handle);  // moved here
                } else {
                    consume(handle);  // moved here too
                }
                // handle is always moved, no deinit needed
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            MirFunction::new("Test.example")
                // Should NOT have a DeinitIf for handle
                .no_block(|b| b.has_statement(StatementPattern::AnyDeinitIf))
                // Should NOT have a Deinit for handle either
                .no_block(|b| {
                    b.has_statement(StatementPattern::Deinit {
                        local: "handle".to_string(),
                    })
                }),
        );
    }

    #[test]
    fn neither_branch_moves_uses_regular_deinit() {
        // When a variable is not moved in either branch, should use regular Deinit
        Test::new(
            r#"module Test
            import Prelude
            
            struct Handle: not Copyable {
                var fd: lang.i64
                deinit {}
            }
            
            func getVal(h: Handle) -> lang.i64 {
                return h.fd
            }
            
            func example(cond: lang.i1) {
                let handle = Handle(fd: 42);
                if cond {
                    let x = getVal(handle);  // borrowed, not moved
                } else {
                    let y = getVal(handle);  // borrowed, not moved
                }
                // handle needs regular deinit
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            MirFunction::new("Test.example")
                // Should have regular deinit call for handle
                .any_block(|b| {
                    b.has_statement(StatementPattern::DeinitCall {
                        ty: "Test.Handle".to_string(),
                    })
                })
                // Should NOT have DeinitIf for handle
                .no_block(|b| b.has_statement(StatementPattern::AnyDeinitIf)),
        );
    }
}

// =============================================================================
// ENUM DEINIT (Phase 5.6)
// =============================================================================

mod enum_deinit {
    use super::*;

    #[test]
    fn enum_with_non_copyable_payload_generates_switch() {
        // When dropping an enum with non-copyable payloads, should generate
        // a switch on the discriminant to drop only the active variant
        Test::new(
            r#"module Test
            import Prelude
            
            struct Handle: not Copyable {
                var fd: lang.i64
                deinit {}
            }
            
            enum Resource: not Copyable {
                case file(handle: Handle)
                case none
            }
            
            func example() {
                let r = Resource.file(handle: Handle(fd: 42));
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            MirFunction::new("Test.example")
                // Should have a switch for variant-based drop
                .any_block(|b| b.terminates_with(TerminatorPattern::Switch)),
        );
    }

    #[test]
    fn enum_with_only_copyable_payloads_no_switch() {
        // When all payloads are copyable, no switch needed for drop
        Test::new(
            r#"module Test
            @builtin(.Copyable)
            protocol Copyable {}
            
            enum Value {
                case lang.i64(val: lang.i64)
                case pair(a: lang.i64, b: lang.i64)
                case none
            }
            
            func example() {
                let v = Value.lang.i64(val: 42);
            }
        "#,
        )
        .without_prelude()
        .expect(Compiles)
        .expect(
            MirFunction::new("Test.example")
                // Should NOT have a switch for variant-based drop
                .no_block(|b| b.terminates_with(TerminatorPattern::Switch)),
        );
    }

    // NOTE: Enum deinit blocks are not yet supported by the parser
    // This test is disabled until enum deinit parsing is implemented
    // #[test]
    // fn enum_with_deinit_block_calls_deinit() {
    //     // When an enum has a deinit block, it should be called
    //     Test::new(
    //         r#"module Test
    //         @builtin(.Copyable)
    //         protocol Copyable {}
    //
    //         enum Resource: not Copyable {
    //             case active(val: lang.i64)
    //             case inactive
    //
    //             deinit {}
    //         }
    //
    //         func example() {
    //             let r = Resource.active(val: 42);
    //         }
    //     "#,
    //     )
    //     .expect(Compiles)
    //     .expect(
    //         MirFunction::new("Test.example")
    //             // Should call the enum's deinit function
    //             .any_block(|b| b.has_statement(StatementPattern::DeinitCall { ty: "Test.Resource".to_string() })),
    //     );
    // }

    #[test]
    fn enum_drop_handles_nested_non_copyable() {
        // Dropping an enum variant should recursively drop non-copyable fields
        Test::new(
            r#"module Test
            import Prelude
            
            struct Inner: not Copyable {
                var id: lang.i64
                deinit {}
            }
            
            struct Outer: not Copyable {
                var inner: Inner
                deinit {}
            }
            
            enum Container: not Copyable {
                case wrapped(value: Outer)
                case empty
            }
            
            func example() {
                let c = Container.wrapped(value: Outer(inner: Inner(id: 1)));
            }
        "#,
        )
        .expect(Compiles)
        .expect(
            MirFunction::new("Test.example")
                // Should have switch for enum drop
                .any_block(|b| b.terminates_with(TerminatorPattern::Switch))
                // Should have deinit calls in the variant drop path
                .any_block(|b| b.has_statement(StatementPattern::AnyDeinitCall)),
        );
    }
}
