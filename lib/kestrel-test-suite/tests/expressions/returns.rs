//! Tests for return expressions.
//!
//! These tests verify that:
//! - Return expressions parse correctly
//! - Bare return (returning unit) works
//! - Return with value works
//! - Return in nested blocks works
//! - Return is statement-like (doesn't require semicolon when trailing)

use kestrel_test_suite::*;

mod return_basic {
    use super::*;

    #[test]
    fn bare_return_compiles() {
        Test::new(
            r#"
module Main

func test() {
    return
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn return_with_integer_compiles() {
        Test::new(
            r#"
module Main

func test() -> Int {
    return 42
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn return_with_expression_compiles() {
        Test::new(
            r#"
module Main

func test(x: Int) -> Int {
    return x + 1
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn return_with_string_compiles() {
        Test::new(
            r#"
module Main

func test() -> String {
    return "hello"
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn return_with_bool_compiles() {
        Test::new(
            r#"
module Main

func test() -> Bool {
    return true
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }
}

mod return_in_control_flow {
    use super::*;

    #[test]
    fn return_in_if_branch() {
        Test::new(
            r#"
module Main

func test(x: Bool) -> Int {
    if x {
        return 1
    }
    return 0
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn return_in_both_if_else_branches() {
        Test::new(
            r#"
module Main

func test(x: Bool) -> Int {
    if x {
        return 1
    } else {
        return 0
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn return_in_while_loop() {
        Test::new(
            r#"
module Main

func test() -> Int {
    var x: Int = 0;
    while x < 10 {
        if x == 5 {
            return x
        }
        x = x + 1;
    }
    return x
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn return_in_loop() {
        Test::new(
            r#"
module Main

func test() -> Int {
    var x: Int = 0;
    loop {
        if x == 10 {
            return x
        }
        x = x + 1;
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }
}

mod return_statement_like {
    use super::*;

    #[test]
    fn return_without_semicolon_at_end() {
        // Return doesn't need semicolon when it's the last thing
        Test::new(
            r#"
module Main

func test() -> Int {
    return 42
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn return_with_semicolon_followed_by_code() {
        // Return with semicolon can be followed by more code (dead code, but valid)
        Test::new(
            r#"
module Main

func test() -> Int {
    return 42;
    let x: Int = 1;
    x
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }
}

mod return_with_complex_expressions {
    use super::*;

    #[test]
    fn return_with_function_call() {
        Test::new(
            r#"
module Main

func helper() -> Int {
    42
}

func test() -> Int {
    return helper()
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn return_with_binary_expression() {
        Test::new(
            r#"
module Main

func test(a: Int, b: Int) -> Int {
    return a * b + 1
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn return_with_local_variable() {
        Test::new(
            r#"
module Main

func test() -> Int {
    let x: Int = 42;
    return x
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn return_with_grouping() {
        Test::new(
            r#"
module Main

func test() -> Int {
    return (1 + 2) * 3
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }
}

mod return_nested {
    use super::*;

    #[test]
    fn return_in_nested_if() {
        Test::new(
            r#"
module Main

func test(a: Bool, b: Bool) -> Int {
    if a {
        if b {
            return 1
        } else {
            return 2
        }
    } else {
        return 3
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn return_in_nested_loop() {
        Test::new(
            r#"
module Main

func test() -> Int {
    var i: Int = 0;
    while i < 10 {
        var j: Int = 0;
        while j < 10 {
            if i * j == 25 {
                return i + j
            }
            j = j + 1;
        }
        i = i + 1;
    }
    return 0
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn multiple_returns_in_function() {
        Test::new(
            r#"
module Main

func test(x: Int) -> Int {
    if x < 0 {
        return -1
    }
    if x == 0 {
        return 0
    }
    return 1
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }
}
