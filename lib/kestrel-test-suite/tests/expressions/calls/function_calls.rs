//! Tests for function call expressions.
//!
//! These tests verify that function calls are correctly resolved,
//! including standalone functions, nested calls, parameter matching,
//! labeled arguments, and error cases.

use kestrel_test_suite::*;

mod function_calls {
    use super::*;

    // === Basic Function Calls ===

    #[test]
    fn call_simple_function_no_params() {
        // Call a simple zero-parameter function
        Test::new(
            r#"
module Main

func getNumber() -> lang.i64 {
    42
}

func test() -> lang.i64 {
    getNumber()
}
"#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("getNumber")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0)),
        )
        .expect(
            Symbol::new("test")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0)),
        );
    }

    #[test]
    fn call_function_with_single_param() {
        // Call a function with exactly one parameter
        Test::new(
            r#"
module Main

func double(x: lang.i64) -> lang.i64 {
    42
}

func test() -> lang.i64 {
    double(21)
}
"#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("double")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(1)),
        );
    }

    #[test]
    fn call_function_with_multiple_params() {
        // Call a function with multiple parameters - verify arity
        Test::new(
            r#"
module Main

func add(x: lang.i64, y: lang.i64) -> lang.i64 {
    42
}

func test() -> lang.i64 {
    add(1, 2)
}
"#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Main.add")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(2)),
        );
    }

    #[test]
    fn call_three_param_function() {
        // Call a function with three parameters
        Test::new(
            r#"
module Main

func combine(a: lang.i64, b: lang.i64, c: lang.i64) -> lang.i64 {
    42
}

func test() -> lang.i64 {
    combine(1, 2, 3)
}
"#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("combine")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(3)),
        );
    }

    // === Labeled Arguments ===

    #[test]
    fn call_with_single_labeled_argument() {
        // Call a function with one labeled parameter
        Test::new(
            r#"
module Main

func greet(with name: lang.str) -> lang.str {
    name
}

func test() -> lang.str {
    greet(with: "world")
}
"#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("greet")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(1)),
        );
    }

    #[test]
    fn call_with_multiple_labeled_arguments() {
        // Call a function with multiple labeled parameters - verify all match
        Test::new(
            r#"
module Main

func createPoint(x xVal: lang.i64, y yVal: lang.i64) -> lang.i64 {
    42
}

func test() -> lang.i64 {
    createPoint(x: 10, y: 20)
}
"#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("createPoint")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(2)),
        );
    }

    #[test]
    fn call_with_mixed_labeled_unlabeled_args() {
        // Mix of labeled (required label) and unlabeled (no label required) arguments
        Test::new(
            r#"
module Main

func format(value: lang.i64, with prefix: lang.str) -> lang.str {
    prefix
}

func test() -> lang.str {
    format(42, with: "Result: ")
}
"#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("format")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(2)),
        );
    }

    #[test]
    fn call_with_three_labeled_arguments() {
        // Verify function with 3 labeled parameters and correct call
        Test::new(
            r#"
module Main

func build(first x: lang.i64, second y: lang.i64, third z: lang.i64) -> lang.i64 {
    42
}

func test() -> lang.i64 {
    build(first: 1, second: 2, third: 3)
}
"#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("build")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(3)),
        );
    }

    // === Nested Calls ===

    #[test]
    fn nested_function_calls_two_levels() {
        // Function calls as arguments to other functions
        Test::new(
            r#"
module Main

func double(x: lang.i64) -> lang.i64 {
    42
}

func add(x: lang.i64, y: lang.i64) -> lang.i64 {
    42
}

func test() -> lang.i64 {
    add(double(1), double(2))
}
"#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("Main.add")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(2)),
        )
        .expect(
            Symbol::new("Main.double")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(1)),
        );
    }

    #[test]
    fn deeply_nested_calls_four_levels() {
        // Deeply nested function calls (4 levels)
        Test::new(
            r#"
module Main

func id(x: lang.i64) -> lang.i64 {
    x
}

func test() -> lang.i64 {
    id(id(id(id(42))))
}
"#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("id")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(1)),
        );
    }

    #[test]
    fn mixed_nested_and_labeled_calls() {
        // Nested calls mixed with labeled arguments
        Test::new(
            r#"
module Main

func double(x: lang.i64) -> lang.i64 {
    42
}

func format(value: lang.i64, with prefix: lang.str) -> lang.str {
    prefix
}

func test() -> lang.str {
    format(double(5), with: "Result: ")
}
"#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("format")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(2)),
        )
        .expect(
            Symbol::new("double")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(1)),
        );
    }

    // === Return Type Propagation ===

    #[test]
    fn function_return_type_in_variable_binding() {
        // Function return type is correctly propagated to variable binding
        Test::new(
            r#"
module Main

func getString() -> lang.str {
    "hello"
}

func test() -> lang.str {
    let s: lang.str = getString();
    s
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("getString").is(SymbolKind::Function))
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn void_function_call() {
        // Calling a void (unit) return type function
        Test::new(
            r#"
module Main

func doSomething() -> () {
    ()
}

func test() -> () {
    doSomething()
}
"#,
        )
        .expect(Compiles)
        .expect(
            Symbol::new("doSomething")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0)),
        )
        .expect(
            Symbol::new("test")
                .is(SymbolKind::Function)
                .has(Behavior::ParameterCount(0)),
        );
    }

    // === Error Cases ===

    #[test]
    fn call_with_too_few_arguments_error() {
        // Calling function with fewer arguments than required should error
        Test::new(
            r#"
module Main

func add(x: lang.i64, y: lang.i64) -> lang.i64 {
    42
}

func test() -> lang.i64 {
    add(1)
}
"#,
        )
        .expect(HasError("no matching overload"));
    }

    #[test]
    fn call_with_too_many_arguments_error() {
        // Calling function with more arguments than required should error
        Test::new(
            r#"
module Main

func double(x: lang.i64) -> lang.i64 {
    42
}

func test() -> lang.i64 {
    double(1, 2)
}
"#,
        )
        .expect(HasError("no matching overload"));
    }

    #[test]
    fn call_undefined_function_error() {
        // Calling a function that doesn't exist should produce an error
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    undefined()
}
"#,
        )
        .expect(HasError("undefined name"));
    }

    #[test]
    fn call_with_wrong_labeled_argument_error() {
        // Using incorrect parameter label should produce an error
        Test::new(
            r#"
module Main

func greet(with name: lang.str) -> lang.str {
    name
}

func test() -> lang.str {
    greet(using: "world")
}
"#,
        )
        .expect(HasError("no matching overload"));
    }

    #[test]
    fn call_with_missing_required_label_error() {
        // Omitting a required parameter label should error
        Test::new(
            r#"
module Main

func format(value: lang.i64, with prefix: lang.str) -> lang.str {
    prefix
}

func test() -> lang.str {
    format(42, "Result: ")
}
"#,
        )
        .expect(HasError("no matching overload"));
    }
}
