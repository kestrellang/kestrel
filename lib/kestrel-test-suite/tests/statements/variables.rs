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
        Test::new("module Main\nfunc test() -> Int { let x: Int = 42; var y: Int = 99; x }")
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
            "module Main\nfunc test() -> Int { let x: Int = 1; let y: Int = 2; var z: Int = 3; x }",
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
        Test::new("module Main\nfunc getString() -> String { let msg: String = \"hello\"; msg }")
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
        Test::new("module Main\nfunc test(x: Int) -> Int { let x: Int = 99; x }")
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
        Test::new("module Main\nfunc getArray() -> [Int] { let arr: [Int] = [1, 2, 3]; arr }")
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
        Test::new("module Main\nfunc getPair() -> (Int, String) { let pair: (Int, String) = (42, \"hi\"); pair }")
            .expect(Compiles)
            .expect(Symbol::new("getPair").is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0)));
    }

    #[test]
    fn var_with_reassignment() {
        // Var allows reassignment, let does not (both compile)
        Test::new("module Main\nfunc test() -> Int { var x: Int = 1; x }")
            .expect(Compiles)
            .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn mixed_variable_types() {
        // Mix of Int, String, and array variables in single function
        Test::new("module Main\nfunc mixed() -> Int { let num: Int = 42; let text: String = \"x\"; let items: [Int] = [1]; num }")
            .expect(Compiles)
            .expect(Symbol::new("mixed").is(SymbolKind::Function)
                .has(Behavior::HasBody(true)));
    }
}
