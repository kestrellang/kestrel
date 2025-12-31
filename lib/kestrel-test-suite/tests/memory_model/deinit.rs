//! Tests for deinit semantics (Phase 5)
//!
//! This module tests the deinit implementation including:
//! - Deinit parsing and semantic binding
//! - DeinitBehavior attachment to parent structs
//! - Duplicate deinit error detection
//! - Copyable + deinit warning
//!
//! NOTE: Currently, deinit bodies with statements (e.g., `deinit { let x = 1 }`)
//! have a parser bug that causes tree building to fail. Tests use empty deinit
//! bodies `deinit {}` until the parser bug is fixed.

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
            @builtin(.Copyable)
            protocol Copyable {}
            
            struct Handle: not Copyable {
                var fd: Int
                
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
                var x: Int
                var y: Int
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
            @builtin(.Copyable)
            protocol Copyable {}
            
            struct Resource: not Copyable {
                var id: Int
                
                init(id: Int) {
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
            @builtin(.Copyable)
            protocol Copyable {}
            
            struct Handle: not Copyable {
                var fd: Int
                
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
            @builtin(.Copyable)
            protocol Copyable {}
            
            struct Handle: not Copyable {
                var fd: Int
                
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
            @builtin(.Copyable)
            protocol Copyable {}
            
            struct Handle: not Copyable {
                var fd: Int
                
                deinit {}
                deinit {}
            }
        "#,
        )
        .expect(HasError("already has a deinit"));
    }
}

// =============================================================================
// COPYABLE + DEINIT WARNING
// =============================================================================

mod copyable_with_deinit {
    use super::*;

    #[test]
    fn copyable_struct_with_deinit_warning() {
        // A Copyable struct with deinit should emit a warning
        Test::new(
            r#"module Test
            struct Counter {
                var count: Int
                
                deinit {}
            }
        "#,
        )
        .expect(Compiles) // Should compile (warning, not error)
        .expect(HasWarning("is Copyable but has deinit"))
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
            @builtin(.Copyable)
            protocol Copyable {}
            
            struct Handle: not Copyable {
                var fd: Int
                
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
            @builtin(.Copyable)
            protocol Copyable {}
            
            struct Handle: not Copyable {
                var fd: Int
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
            @builtin(.Copyable)
            protocol Copyable {}
            
            struct Connection: not Copyable {
                var host: String
                var port: Int
                var connected: Bool
                
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
            @builtin(.Copyable)
            protocol Copyable {}
            
            protocol Resource {}
            
            struct Handle: Resource, not Copyable {
                var fd: Int
                
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
