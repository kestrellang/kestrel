//! Tests for loop expressions (while, loop, break, continue).
//!
//! These tests verify that:
//! - While loops parse and resolve correctly
//! - Loop (infinite) loops work
//! - Break and continue work within loops
//! - Labeled break/continue work for nested loops
//! - Break/continue outside loops produce errors
//! - Edge cases and error conditions are handled correctly

use kestrel_test_suite::*;

mod while_basic {
    use super::*;

    #[test]
    fn simple_while_loop() {
        Test::new(
            r#"
module Main

func test() {
    var x: lang.i64 = 0;
    while lang.i64_signed_lt(x, 10) {
        x = lang.i64_add(x, 1);
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn while_without_semicolon_followed_by_expression() {
        // While loops are statement-like and don't need semicolons
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    var x: lang.i64 = 0;
    while lang.i64_signed_lt(x, 5) {
        x = lang.i64_add(x, 1);
    }
    x
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn multiple_while_without_semicolons() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    var x: lang.i64 = 0;
    while lang.i64_signed_lt(x, 5) {
        x = lang.i64_add(x, 1);
    }
    while lang.i64_signed_lt(x, 10) {
        x = lang.i64_add(x, 1);
    }
    x
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn while_with_complex_condition() {
        Test::new(
            r#"
module Main

func test(a: lang.i1, b: lang.i1) {
    while lang.i1_and(a, b) {
        ()
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn nested_while_loops() {
        Test::new(
            r#"
module Main

func test() {
    var i: lang.i64 = 0;
    var j: lang.i64 = 0;
    while lang.i64_signed_lt(i, 10) {
        j = 0;
        while lang.i64_signed_lt(j, 10) {
            j = lang.i64_add(j, 1);
        }
        i = lang.i64_add(i, 1);
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }
}

mod loop_basic {
    use super::*;

    #[test]
    fn simple_loop_with_break() {
        Test::new(
            r#"
module Main

func test() {
    loop {
        break;
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn loop_without_semicolon_followed_by_expression() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    var x: lang.i64 = 0;
    loop {
        x = lang.i64_add(x, 1);
        if lang.i64_signed_gt(x, 5) {
            break;
        }
    }
    x
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn nested_loops() {
        Test::new(
            r#"
module Main

func test() {
    loop {
        loop {
            break;
        }
        break;
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }
}

mod break_continue {
    use super::*;

    #[test]
    fn break_in_while() {
        Test::new(
            r#"
module Main

func test() {
    while true {
        break;
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn continue_in_while() {
        Test::new(
            r#"
module Main

func test() {
    var x: lang.i64 = 0;
    while lang.i64_signed_lt(x, 10) {
        x = lang.i64_add(x, 1);
        continue;
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn break_in_loop() {
        Test::new(
            r#"
module Main

func test() {
    loop {
        break;
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn continue_in_loop() {
        Test::new(
            r#"
module Main

func test() {
    var x: lang.i64 = 0;
    loop {
        x = lang.i64_add(x, 1);
        if lang.i64_signed_gt(x, 10) {
            break;
        }
        continue;
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn break_outside_loop_fails() {
        Test::new(
            r#"
module Main

func test() {
    break;
}
"#,
        )
        .expect(Fails)
        .expect(HasError("outside of loop"));
    }

    #[test]
    fn continue_outside_loop_fails() {
        Test::new(
            r#"
module Main

func test() {
    continue;
}
"#,
        )
        .expect(Fails)
        .expect(HasError("outside of loop"));
    }

    #[test]
    fn break_in_if_outside_loop_fails() {
        Test::new(
            r#"
module Main

func test() {
    if true {
        break;
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("outside of loop"));
    }
}

mod labeled_loops {
    use super::*;

    #[test]
    fn labeled_while_with_break() {
        Test::new(
            r#"
module Main

func test() {
    outer: while true {
        while true {
            break outer;
        }
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn labeled_loop_with_break() {
        Test::new(
            r#"
module Main

func test() {
    outer: loop {
        loop {
            break outer;
        }
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn labeled_while_with_continue() {
        Test::new(
            r#"
module Main

func test() {
    var x: lang.i64 = 0;
    outer: while lang.i64_signed_lt(x, 100) {
        x = lang.i64_add(x, 1);
        while true {
            continue outer;
        }
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn break_to_undeclared_label_fails() {
        Test::new(
            r#"
module Main

func test() {
    while true {
        break nonexistent;
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("undeclared label"));
    }

    #[test]
    fn continue_to_undeclared_label_fails() {
        Test::new(
            r#"
module Main

func test() {
    while true {
        continue nonexistent;
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("undeclared label"));
    }

    #[test]
    fn multiple_labeled_loops() {
        Test::new(
            r#"
module Main

func test() {
    outer: loop {
        inner: loop {
            break inner;
        }
        break outer;
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }
}

mod loop_scoping {
    use super::*;

    #[test]
    fn variable_in_while_block_not_visible_outside() {
        Test::new(
            r#"
module Main

func test() {
    while true {
        let x: lang.i64 = 42;
        break;
    }
    x
}
"#,
        )
        .expect(Fails)
        .expect(HasError("undefined"));
    }

    #[test]
    fn variable_in_loop_block_not_visible_outside() {
        Test::new(
            r#"
module Main

func test() {
    loop {
        let y: lang.i64 = 10;
        break;
    }
    y
}
"#,
        )
        .expect(Fails)
        .expect(HasError("undefined"));
    }

    #[test]
    fn outer_variable_visible_inside_loop() {
        Test::new(
            r#"
module Main

func test() {
    let outer: lang.i64 = 10;
    while lang.i64_signed_gt(outer, 0) {
        break;
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }
}

mod edge_cases {
    use super::*;

    #[test]
    fn deeply_nested_loops() {
        Test::new(
            r#"
module Main

func test() {
    var a: lang.i64 = 0;
    while lang.i64_signed_lt(a, 10) {
        var b: lang.i64 = 0;
        while lang.i64_signed_lt(b, 10) {
            var c: lang.i64 = 0;
            while lang.i64_signed_lt(c, 10) {
                var d: lang.i64 = 0;
                loop {
                    d = lang.i64_add(d, 1);
                    if lang.i64_signed_gt(d, 5) {
                        break;
                    }
                }
                c = lang.i64_add(c, 1);
            }
            b = lang.i64_add(b, 1);
        }
        a = lang.i64_add(a, 1);
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn loop_inside_if_inside_loop() {
        Test::new(
            r#"
module Main

func test() {
    var x: lang.i64 = 0;
    while lang.i64_signed_lt(x, 10) {
        if lang.i64_signed_lt(x, 5) {
            loop {
                break;
            }
        }
        x = lang.i64_add(x, 1);
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn if_inside_loop_inside_if() {
        Test::new(
            r#"
module Main

func test() {
    var x: lang.i64 = 0;
    if true {
        while lang.i64_signed_lt(x, 10) {
            if lang.i64_eq(x, 5) {
                break;
            }
            x = lang.i64_add(x, 1);
        }
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn empty_while_body() {
        Test::new(
            r#"
module Main

func test() {
    var x: lang.i64 = 0;
    while lang.i64_signed_lt(x, 0) {
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn empty_loop_body_with_break() {
        Test::new(
            r#"
module Main

func test() {
    loop {
        break;
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn while_with_comparison_condition() {
        Test::new(
            r#"
module Main

func test() {
    var counter: lang.i64 = 0;
    while lang.i64_signed_lt(counter, 10) {
        counter = lang.i64_add(counter, 1);
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn while_with_function_call_condition() {
        Test::new(
            r#"
module Main

func shouldContinue() -> lang.i1 {
    false
}

func test() {
    while shouldContinue() {
        ()
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn while_with_nested_function_call_condition() {
        Test::new(
            r#"
module Main

func getValue() -> lang.i64 {
    5
}

func isValid(x: lang.i64) -> lang.i1 {
    lang.i64_signed_gt(x, 0)
}

func test() {
    while isValid(getValue()) {
        break;
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn if_with_function_call_condition() {
        Test::new(
            r#"
module Main

func check() -> lang.i1 {
    true
}

func test() {
    if check() {
        ()
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn break_as_last_statement() {
        Test::new(
            r#"
module Main

func test() {
    var x: lang.i64 = 0;
    while true {
        x = lang.i64_add(x, 1);
        break
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn continue_as_last_statement() {
        Test::new(
            r#"
module Main

func test() {
    var x: lang.i64 = 0;
    while lang.i64_signed_lt(x, 10) {
        x = lang.i64_add(x, 1);
        continue
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn multiple_breaks_in_loop() {
        Test::new(
            r#"
module Main

func test() {
    var x: lang.i64 = 0;
    loop {
        x = lang.i64_add(x, 1);
        if lang.i64_eq(x, 5) {
            break;
        }
        if lang.i64_eq(x, 10) {
            break;
        }
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn multiple_continues_in_loop() {
        Test::new(
            r#"
module Main

func test() {
    var x: lang.i64 = 0;
    while lang.i64_signed_lt(x, 20) {
        x = lang.i64_add(x, 1);
        if lang.i64_eq(x, 5) {
            continue;
        }
        if lang.i64_eq(x, 10) {
            continue;
        }
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn break_and_continue_in_same_loop() {
        Test::new(
            r#"
module Main

func test() {
    var x: lang.i64 = 0;
    while lang.i64_signed_lt(x, 20) {
        x = lang.i64_add(x, 1);
        if lang.i64_eq(x, 5) {
            continue;
        }
        if lang.i64_eq(x, 15) {
            break;
        }
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn labeled_break_from_deeply_nested() {
        Test::new(
            r#"
module Main

func test() {
    outermost: while true {
        while true {
            loop {
                break outermost;
            }
        }
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn same_label_in_different_scopes() {
        // Labels are scoped to their containing loop, so reusing a label name
        // after the previous loop ends should be fine
        Test::new(
            r#"
module Main

func test() {
    myloop: while true {
        break myloop;
    }
    myloop: loop {
        break myloop;
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }
}

mod error_cases {
    use super::*;

    #[test]
    fn break_in_function_body_no_loop() {
        Test::new(
            r#"
module Main

func test() {
    let x: lang.i64 = 1;
    break;
    let y: lang.i64 = 2;
}
"#,
        )
        .expect(Fails)
        .expect(HasError("outside of loop"));
    }

    #[test]
    fn continue_in_function_body_no_loop() {
        Test::new(
            r#"
module Main

func test() {
    let x: lang.i64 = 1;
    continue;
    let y: lang.i64 = 2;
}
"#,
        )
        .expect(Fails)
        .expect(HasError("outside of loop"));
    }

    #[test]
    fn break_in_nested_if_no_loop() {
        Test::new(
            r#"
module Main

func test() {
    if true {
        if true {
            break;
        }
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("outside of loop"));
    }

    #[test]
    fn continue_in_nested_if_no_loop() {
        Test::new(
            r#"
module Main

func test() {
    if true {
        if false {
            continue;
        }
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("outside of loop"));
    }

    #[test]
    fn break_to_label_that_exists_but_not_in_scope() {
        // Label exists in a sibling loop, not an enclosing loop
        Test::new(
            r#"
module Main

func test() {
    sibling: while true {
        break;
    }
    while true {
        break sibling;
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("undeclared label"));
    }

    #[test]
    fn continue_to_label_that_exists_but_not_in_scope() {
        Test::new(
            r#"
module Main

func test() {
    var x: lang.i64 = 0;
    sibling: while lang.i64_signed_lt(x, 10) {
        x = lang.i64_add(x, 1);
    }
    while lang.i64_signed_lt(x, 20) {
        x = lang.i64_add(x, 1);
        continue sibling;
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("undeclared label"));
    }

    #[test]
    fn break_to_inner_label_from_outer() {
        // Can't break to a label that's inside the current scope, only enclosing
        Test::new(
            r#"
module Main

func test() {
    while true {
        break inner;
        inner: loop {
            break;
        }
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("undeclared label"));
    }

    #[test]
    fn break_with_typo_in_label() {
        Test::new(
            r#"
module Main

func test() {
    myloop: while true {
        break mylooop;
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("undeclared label"));
    }

    #[test]
    fn continue_with_typo_in_label() {
        Test::new(
            r#"
module Main

func test() {
    var x: lang.i64 = 0;
    myloop: while lang.i64_signed_lt(x, 10) {
        x = lang.i64_add(x, 1);
        continue myloooop;
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("undeclared label"));
    }

    #[test]
    fn use_loop_variable_after_loop() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    while true {
        let counter: lang.i64 = 0;
        break;
    }
    counter
}
"#,
        )
        .expect(Fails)
        .expect(HasError("undefined"));
    }

    #[test]
    fn use_nested_loop_variable_outside() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    while true {
        loop {
            let inner: lang.i64 = 42;
            break;
        }
        inner
    }
}
"#,
        )
        .expect(Fails)
        .expect(HasError("undefined"));
    }

    #[test]
    fn shadowed_variable_not_visible_after_loop() {
        Test::new(
            r#"
module Main

func test() -> lang.i64 {
    let x: lang.i64 = 1;
    while true {
        let x: lang.i64 = 2;
        break;
    }
    x
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }
}

mod complex_control_flow {
    use super::*;

    #[test]
    fn while_with_else_if_chain_inside() {
        Test::new(
            r#"
module Main

func test() {
    var x: lang.i64 = 0;
    while lang.i64_signed_lt(x, 100) {
        if lang.i64_signed_lt(x, 10) {
            x = lang.i64_add(x, 1);
        } else if lang.i64_signed_lt(x, 50) {
            x = lang.i64_add(x, 5);
        } else {
            x = lang.i64_add(x, 10);
        }
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn nested_labeled_loops_with_mixed_breaks() {
        Test::new(
            r#"
module Main

func test() {
    var x: lang.i64 = 0;
    outer: while lang.i64_signed_lt(x, 100) {
        var y: lang.i64 = 0;
        middle: loop {
            var z: lang.i64 = 0;
            inner: while lang.i64_signed_lt(z, 10) {
                z = lang.i64_add(z, 1);
                if lang.i64_eq(z, 5) {
                    break inner;
                }
                if lang.i64_eq(z, 7) {
                    break middle;
                }
            }
            y = lang.i64_add(y, 1);
            if lang.i64_signed_gt(y, 3) {
                break outer;
            }
        }
        x = lang.i64_add(x, 1);
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn loop_with_conditional_break_and_continue() {
        Test::new(
            r#"
module Main

func test() {
    var i: lang.i64 = 0;
    outer: loop {
        i = lang.i64_add(i, 1);
        var j: lang.i64 = 0;
        while lang.i64_signed_lt(j, i) {
            j = lang.i64_add(j, 1);
            if lang.i64_eq(j, 3) {
                continue;
            }
            if lang.i64_eq(j, 5) {
                continue outer;
            }
        }
        if lang.i64_signed_gt(i, 10) {
            break outer;
        }
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn three_levels_of_labeled_loops() {
        Test::new(
            r#"
module Main

func test() {
    a: while true {
        b: while true {
            c: while true {
                break a;
            }
            break b;
        }
        break;
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn alternating_while_and_loop() {
        Test::new(
            r#"
module Main

func test() {
    var x: lang.i64 = 0;
    while lang.i64_signed_lt(x, 5) {
        loop {
            var y: lang.i64 = 0;
            while lang.i64_signed_lt(y, 3) {
                loop {
                    break;
                }
                y = lang.i64_add(y, 1);
            }
            break;
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

mod statements_after_loops {
    use super::*;

    #[test]
    fn statement_after_while_in_while() {
        Test::new(
            r#"
module Main

func test() {
    var x: lang.i64 = 0;
    while lang.i64_signed_lt(x, 10) {
        while lang.i64_signed_lt(x, 5) {
            x = lang.i64_add(x, 1);
        }
        x = lang.i64_add(x, 1);
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn statement_after_loop_in_while() {
        Test::new(
            r#"
module Main

func test() {
    var x: lang.i64 = 0;
    while lang.i64_signed_lt(x, 10) {
        loop {
            break;
        }
        x = lang.i64_add(x, 1);
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn statement_after_if_in_while() {
        Test::new(
            r#"
module Main

func test() {
    var x: lang.i64 = 0;
    while lang.i64_signed_lt(x, 10) {
        if lang.i64_signed_lt(x, 5) {
            x = lang.i64_add(x, 2);
        }
        x = lang.i64_add(x, 1);
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn multiple_statement_like_expressions_in_sequence() {
        Test::new(
            r#"
module Main

func test() {
    var x: lang.i64 = 0;
    while lang.i64_signed_lt(x, 100) {
        if lang.i64_signed_lt(x, 10) {
            x = lang.i64_add(x, 1);
        }
        while lang.i64_signed_lt(x, 20) {
            x = lang.i64_add(x, 1);
        }
        loop {
            x = lang.i64_add(x, 1);
            break;
        }
        if lang.i64_signed_gt(x, 50) {
            break;
        }
        x = lang.i64_add(x, 1);
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn statement_after_while_in_loop() {
        Test::new(
            r#"
module Main

func test() {
    var x: lang.i64 = 0;
    loop {
        while lang.i64_signed_lt(x, 5) {
            x = lang.i64_add(x, 1);
        }
        x = lang.i64_add(x, 1);
        if lang.i64_signed_gt(x, 10) {
            break;
        }
    }
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("test").is(SymbolKind::Function));
    }

    #[test]
    fn statement_after_nested_if_while_loop() {
        Test::new(
            r#"
module Main

func test() {
    var x: lang.i64 = 0;
    while lang.i64_signed_lt(x, 100) {
        if true {
            while lang.i64_signed_lt(x, 10) {
                loop {
                    break;
                }
                x = lang.i64_add(x, 1);
            }
            x = lang.i64_add(x, 1);
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

mod regression {
    use super::*;

    #[test]
    fn implicit_member_after_while_not_parsed_as_member_access() {
        // Regression test for: `while true` with unreachable code after loop causes issues
        // Previously, `.None` on a newline after a while expression would be parsed
        // as a member access on the while expression (which returns unit), causing
        // "undefined name 'None'" error.
        // Fixed by not skipping trivia before the dot in postfix member access,
        // requiring the dot to be on the same line as the receiver.
        Test::new(
            r#"
module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func find(limit: lang.i64) -> Option[lang.i64] {
    var x: lang.i64 = 0;

    while true {
        if lang.i64_signed_gt(x, limit) {
            return .Some(x)
        }
        x = lang.i64_add(x, 1);
    }

    // This is unreachable code (after an infinite loop),
    // but should not cause a parse/binding error
    .None
}
"#,
        )
        .expect(Compiles)
        .expect(Symbol::new("find").is(SymbolKind::Function));
    }
}
