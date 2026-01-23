//! Tests for assignment expressions.
//!
//! These tests verify that assignment statements are correctly resolved.

use kestrel_test_suite::*;

mod assignment_expressions {
    use super::*;

    #[test]
    fn basic_assignment_to_var() {
        // Basic assignment to a mutable variable with lang.i64 literals
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    var x: lang.i64 = 0;
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

func getValue() -> lang.i64 { 42 }

func test() -> lang.i64 {
    var x: lang.i64 = 0;
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

func test() -> [lang.i64] {
    var arr: [lang.i64] = [];
    arr = [1, 2, 3];
    arr
}
"#,
        )
        .with_stdlib()
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

func test() -> lang.i64 {
    var x: lang.i64 = 0;
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

func test(value: lang.i64) -> lang.i64 {
    var x: lang.i64 = 0;
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

func test() -> lang.str {
    var msg: lang.str = "";
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

func test() -> lang.i64 {
    var num: lang.i64 = 0;
    num = 42;
    var text: lang.str = "";
    text = "assigned";
    var items: [lang.i64] = [];
    items = [1, 2];
    num
}
"#,
        )
        .with_stdlib()
        .expect(Compiles)
        .expect(
            Symbol::new("test")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0))
                .has(Behavior::HasBody(true)),
        );
    }

    #[test]
    fn assignment_expression_result() {
        // TODO: Enable once assignment expressions are implemented
        Test::new(
            r#"
module Main
func test() {
    var x = 0;
    let y = (x = 42);
}
"#,
        )
        .expect(Compiles);
    }
}
