//! Tests for control flow expressions (if/else).
//!
//! These tests verify that:
//! - If expressions parse correctly
//! - Else branches work
//! - Else-if chaining works
//! - Variables in if blocks have proper scoping (not visible outside)
//! - If expressions can be used as values

use kestrel_test_suite::*;

mod if_basic {
    use super::*;

    #[test]
    fn if_without_else_compiles() {
        Test::new(
            r#"
module Main

func test() {
    if true {
        1
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn if_without_semicolon_followed_by_expression() {
        // If expressions don't need semicolons - they are statement-like
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    if false {
        1
    }
    42
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn multiple_if_without_semicolons() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    if false {
        1
    }
    if false {
        2
    }
    if true {
        3
    } else {
        4
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn if_with_else_compiles() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    if true {
        1
    } else {
        2
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn if_else_if_chain_compiles() {
        Test::new(
            r#"
module Main

func test(x: lang.i64) -> lang.i64 {
    if lang.i64_eq(x, 1) {
        10
    } else if lang.i64_eq(x, 2) {
        20
    } else if lang.i64_eq(x, 3) {
        30
    } else {
        0
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn if_with_complex_condition() {
        Test::new(
            r#"
module Main

func test(a: lang.i1, b: lang.i1) -> lang.i64 {
    if lang.i1_and(a, b) {
        1
    } else {
        0
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn nested_if_expressions() {
        Test::new(
            r#"
module Main

func test(a: lang.i1, b: lang.i1) -> lang.i64 {
    if a {
        if b {
            1
        } else {
            2
        }
    } else {
        3
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }
}

mod if_scoping {
    use super::*;

    #[test]
    fn variable_in_if_block_not_visible_outside() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    if true {
        let x: lang.i64 = 42;
        x
    }
    x
}
"#,
        )
        .expect(Fails)
        .expect(HasError("undefined"));
    }

    #[test]
    fn variable_in_else_block_not_visible_outside() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    if true {
        1
    } else {
        let y: lang.i64 = 10;
        y
    }
    y
}
"#,
        )
        .expect(Fails)
        .expect(HasError("undefined"));
    }

    #[test]
    fn variable_visible_within_its_block() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    if true {
        let x: lang.i64 = 5;
        let y: lang.i64 = lang.i64_add(x, 1);
        y
    } else {
        0
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn outer_variable_visible_inside_if() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    let outer: lang.i64 = 10;
    if true {
        lang.i64_add(outer, 5)
    } else {
        lang.i64_sub(outer, 5)
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn shadowing_inside_if_block() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    let x: lang.i64 = 100;
    if true {
        let x: lang.i64 = 1;
        x
    } else {
        x
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }
}

mod if_with_statements {
    use super::*;

    #[test]
    fn if_block_with_multiple_statements() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    if true {
        let a: lang.i64 = 1;
        let b: lang.i64 = 2;
        let c: lang.i64 = lang.i64_add(a, b);
        c
    } else {
        0
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn if_block_with_side_effects() {
        // Test that we can have side effects (assignments) in an if block
        Test::new(
            r#"
module Main

func test() {
    var localX: lang.i64 = 0;
    if true {
        localX = 10;
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }
}

mod never_type_propagation {
    use super::*;

    #[test]
    fn if_with_return_in_else_branch() {
        // The else branch returns Never, so the if expression type is lang.i64
        Test::new(
            r#"
module Main

func test(cond: lang.i1) -> lang.i64 {
    if cond {
        42
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
    fn if_with_return_in_then_branch() {
        // The then branch returns Never, so the if expression type is lang.i64
        Test::new(
            r#"
module Main

func test(cond: lang.i1) -> lang.i64 {
    if cond {
        return 0
    } else {
        42
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn if_with_return_in_both_branches() {
        // Both branches return Never, so the if expression type is Never
        // This is fine because the function still returns the correct type
        Test::new(
            r#"
module Main

func test(cond: lang.i1) -> lang.i64 {
    if cond {
        return 1
    } else {
        return 2
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn if_else_if_with_return() {
        // Test Never propagation through else-if chains
        Test::new(
            r#"
module Main

func test(x: lang.i64) -> lang.i64 {
    if lang.i64_eq(x, 1) {
        return 10
    } else if lang.i64_eq(x, 2) {
        20
    } else {
        30
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn if_else_if_chain_all_return() {
        // All branches return, which is valid
        Test::new(
            r#"
module Main

func test(x: lang.i64) -> lang.i64 {
    if lang.i64_eq(x, 1) {
        return 10
    } else if lang.i64_eq(x, 2) {
        return 20
    } else {
        return 30
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn nested_if_with_never_propagation() {
        // Never propagates through nested if expressions
        Test::new(
            r#"
module Main

func test(a: lang.i1, b: lang.i1) -> lang.i64 {
    if a {
        if b {
            return 1
        } else {
            2
        }
    } else {
        3
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn if_as_expression_with_never_in_else() {
        // Using if as an expression where else returns Never
        Test::new(
            r#"
module Main

func test(cond: lang.i1) -> lang.i64 {
    let x: lang.i64 = if cond { 42 } else { return 0 };
    x
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn if_as_expression_with_never_in_then() {
        // Using if as an expression where then returns Never
        Test::new(
            r#"
module Main

func test(cond: lang.i1) -> lang.i64 {
    let x: lang.i64 = if cond { return 0 } else { 42 };
    x
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }
}

mod regression {
    use super::*;

    #[test]
    fn never_type_unifies_with_concrete_types_in_if_else() {
        // Regression test for: `lang.panic` return type `!` doesn't unify with other branch types
        // Issue: When an if-else expression has a concrete type in one branch and `lang.panic`
        // (which returns `!` never type) in the other, the compiler should use the concrete type
        // as the result type (Never is a bottom type and should unify with anything).
        Test::new(
            r#"
module Main

// Test case: Never in else branch with i64
public func test_panic_in_else(condition: lang.i1) -> lang.i64 {
    if condition {
        42
    } else {
        lang.panic("error")
    }
}

// Test case: Never in then branch with i64
public func test_panic_in_then(condition: lang.i1) -> lang.i64 {
    if condition {
        lang.panic("error")
    } else {
        42
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test_panic_in_else").is(SymbolKind::Function))
        .expect(Symbol::new("test_panic_in_then").is(SymbolKind::Function));
    }
}

mod try_expressions {
    use super::*;

    #[test]
    fn try_expression_not_supported() {
        Test::new(
            r#"
module Main

func getValue() -> lang.i64 {
    42
}

func test() {
    let x = try getValue();
}
"#,
        )
        .expect(Fails)
        .expect(HasError("not yet supported"));
    }
}
