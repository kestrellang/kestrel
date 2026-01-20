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

func test() -> lang.i64 {
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

func test(x: lang.i64) -> lang.i64 {
    return lang.i64_add(x, 1)
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

func test() -> lang.str {
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

func test() -> lang.i1 {
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

func test(x: lang.i1) -> lang.i64 {
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

func test(x: lang.i1) -> lang.i64 {
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

func test() -> lang.i64 {
    var x: lang.i64 = 0;
    while lang.i64_signed_lt(x, 10) {
        if lang.i64_eq(x, 5) {
            return x
        }
        x = lang.i64_add(x, 1);
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

func test() -> lang.i64 {
    var x: lang.i64 = 0;
    loop {
        if lang.i64_eq(x, 10) {
            return x
        }
        x = lang.i64_add(x, 1);
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

func test() -> lang.i64 {
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

func test() -> lang.i64 {
    return 42;
    let x: lang.i64 = 1;
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

func helper() -> lang.i64 {
    42
}

func test() -> lang.i64 {
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

func test(a: lang.i64, b: lang.i64) -> lang.i64 {
    return lang.i64_add(lang.i64_mul(a, b), 1)
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

func test() -> lang.i64 {
    let x: lang.i64 = 42;
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

func test() -> lang.i64 {
    return lang.i64_mul(lang.i64_add(1, 2), 3)
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

func test(a: lang.i1, b: lang.i1) -> lang.i64 {
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

func test() -> lang.i64 {
    var i: lang.i64 = 0;
    while lang.i64_signed_lt(i, 10) {
        var j: lang.i64 = 0;
        while lang.i64_signed_lt(j, 10) {
            if lang.i64_eq(lang.i64_mul(i, j), 25) {
                return lang.i64_add(i, j)
            }
            j = lang.i64_add(j, 1);
        }
        i = lang.i64_add(i, 1);
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

func test(x: lang.i64) -> lang.i64 {
    if lang.i64_signed_lt(x, 0) {
        return 1
    }
    if lang.i64_eq(x, 0) {
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
