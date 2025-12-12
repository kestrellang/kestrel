//! Tests for assignment expressions.
//!
//! These tests verify that assignment statements are correctly resolved.

use kestrel_test_suite::*;

mod assignment_expressions {
    use super::*;

    #[test]
    fn basic_assignment_to_var() {
        // Basic assignment to a mutable variable with int literals
        Test::new(
            r#"
module Main

func test() -> Int {
    var x: Int = 0;
    x = 5;
    x
}
"#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("test")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0))
                .has(Behavior::HasBody(true)),
        );
    }

    #[test]
    fn assignment_with_function_call_result() {
        // Assign result of function call to variable, verify both functions
        Test::new(
            r#"
module Main

func getValue() -> Int { 42 }

func test() -> Int {
    var x: Int = 0;
    x = getValue();
    x
}
"#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("getValue")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0))
                .has(Behavior::HasBody(true)),
        )
        .expect(
            Symbol::new("test")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0))
                .has(Behavior::HasBody(true)),
        );
    }

    #[test]
    fn assignment_with_array_literal() {
        // Assign an array literal to array variable
        Test::new(
            r#"
module Main

func test() -> [Int] {
    var arr: [Int] = [];
    arr = [1, 2, 3];
    arr
}
"#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("test")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0))
                .has(Behavior::HasBody(true)),
        );
    }

    #[test]
    fn multiple_sequential_assignments() {
        // Multiple sequential assignments to same variable
        Test::new(
            r#"
module Main

func test() -> Int {
    var x: Int = 0;
    x = 1;
    x = 2;
    x = 3;
    x
}
"#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("test")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0))
                .has(Behavior::HasBody(true)),
        );
    }

    #[test]
    fn assignment_from_function_parameter() {
        // Assign a function parameter value to a local variable
        Test::new(
            r#"
module Main

func test(value: Int) -> Int {
    var x: Int = 0;
    x = value;
    x
}
"#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("test")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(1))
                .has(Behavior::HasBody(true)),
        );
    }

    #[test]
    fn assignment_with_string_value() {
        // Assignment with string literal
        Test::new(
            r#"
module Main

func test() -> String {
    var msg: String = "";
    msg = "hello";
    msg
}
"#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("test")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0))
                .has(Behavior::HasBody(true)),
        );
    }

    #[test]
    fn chained_assignments_with_different_types() {
        // Assignments with various types in sequence
        Test::new(
            r#"
module Main

func test() -> Int {
    var num: Int = 0;
    num = 42;
    var text: String = "";
    text = "assigned";
    var items: [Int] = [];
    items = [1, 2];
    num
}
"#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("test")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0))
                .has(Behavior::HasBody(true)),
        );
    }
}
