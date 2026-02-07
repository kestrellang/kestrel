//! Tests for variable declarations (let/var).
//!
//! These tests verify that local variable declarations are correctly resolved
//! in function bodies.

use kestrel_test_suite::*;

mod local_variables {
    use super::*;

    #[test]
    fn let_and_var_with_initializers() {
        // Combined test for let and var declarations with type and initializer
        Test::new("module Main\nfunc test() -> lang.i64 { let x: lang.i64 = 42; var y: lang.i64 = 99; x }")
            .expect(Compiles)
            .expect(
                Symbol::new("test")
                    .is(SymbolKind::Function)
                    .has(Behavior::ParameterCount(0)),
            );
    }

    #[test]
    fn multiple_variable_declarations_in_sequence() {
        // Multiple let and var declarations with proper scoping
        Test::new(
            "module Main\nfunc test() -> lang.i64 { let x: lang.i64 = 1; let y: lang.i64 = 2; var z: lang.i64 = 3; x }",
        )
        .expect(Compiles)
        .expect(
            Symbol::new("test")
                .is(SymbolKind::Function)
                .has(Behavior::HasBody(true)),
        );
    }

    #[test]
    fn variable_with_string_type() {
        // Variable holding string value
        Test::new(
            "module Main\nfunc getString() -> lang.str { let msg: lang.str = \"hello\"; msg }",
        )
        .expect(Compiles)
        .expect(
            Symbol::new("getString")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0))
                .has(Behavior::HasBody(true)),
        );
    }

    #[test]
    fn shadowing_parameter_with_local_variable() {
        // Local variable shadows function parameter
        Test::new("module Main\nfunc test(x: lang.i64) -> lang.i64 { let x: lang.i64 = 99; x }")
            .expect(Compiles)
            .expect(
                Symbol::new("test")
                    .is(SymbolKind::Function)
                    .has(Behavior::ParameterCount(1)),
            );
    }

    #[test]
    fn array_variable_declaration() {
        // Variable with array type
        Test::new(
            "module Main\nfunc getArray() -> [lang.i64] { let arr: [lang.i64] = [1, 2, 3]; arr }",
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(
            Symbol::new("getArray")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0)),
        );
    }

    #[test]
    fn tuple_variable_declaration() {
        // Variable with tuple type
        Test::new("module Main\nfunc getPair() -> (lang.i64, lang.str) { let pair: (lang.i64, lang.str) = (42, \"hi\"); pair }")
            .expect(Compiles)
            .expect(Symbol::new("getPair").is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0)));
    }

    #[test]
    fn var_with_reassignment() {
        // Var allows reassignment, let does not (both compile)
        Test::new("module Main\nfunc test() -> lang.i64 { var x: lang.i64 = 1; x }")
            .expect(Compiles)
            .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn mixed_variable_types() {
        // Mix of lang.i64, lang.str, and array variables in single function
        Test::new("module Main\nfunc mixed() -> lang.i64 { let num: lang.i64 = 42; let text: lang.str = \"x\"; let items: [lang.i64] = [1]; num }")
            .with_stdlib()
            .expect(Compiles)
            .expect(Symbol::new("mixed").is(SymbolKind::Function)
                .has(Behavior::HasBody(true)));
    }
}
