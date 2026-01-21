//! Control flow MIR tests.
//!
//! Tests for control flow lowering including:
//! - While loops
//! - Loop with break/continue
//! - Early returns
//! - Nested control flow
//! - Labeled loops

use kestrel_test_suite::mir::*;
use kestrel_test_suite::*;

// ============================================================================
// WHILE LOOPS
// ============================================================================

mod while_loops {
    use super::*;

    #[test]
    fn simple_while_loop() {
        // Based on tmp/05_loops.ks
        Test::new(
            r#"
            module Main

            func countdown(n: lang.i64) -> lang.i64 {
                var i = n;
                while lang.i64_signed_gt(i, 0) {
                    i = lang.i64_sub(i, 1);
                }
                i
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.countdown")
                .returns(MirTy::I64)
                .has_at_least_blocks(3) // entry, loop header, loop body, exit
                .any_block(|b| b.terminates_with(TerminatorPattern::Branch)),
        );
    }

    #[test]
    fn factorial() {
        // Based on tmp/05_loops.ks
        Test::new(
            r#"
            module Main

            func factorial(n: lang.i64) -> lang.i64 {
                var result = 1;
                var i = n;
                while lang.i64_signed_gt(i, 1) {
                    result = lang.i64_mul(result, i);
                    i = lang.i64_sub(i, 1);
                }
                result
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.factorial")
                .returns(MirTy::I64)
                .has_local("result", MirTy::I64)
                .has_local("i", MirTy::I64)
                .any_block(|b| b.has_statement(StatementPattern::BinOp(BinOp::MulSigned))),
        );
    }

    #[test]
    fn fibonacci() {
        // Based on tmp/05_loops.ks
        Test::new(
            r#"
            module Main

            func fibonacci(n: lang.i64) -> lang.i64 {
                if lang.i64_signed_le(n, 1) {
                    n
                } else {
                    var a = 0;
                    var b = 1;
                    var i = 2;
                    while lang.i64_signed_le(i, n) {
                        let temp = lang.i64_add(a, b);
                        a = b;
                        b = temp;
                        i = lang.i64_add(i, 1);
                    }
                    b
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.fibonacci")
                .returns(MirTy::I64)
                .has_at_least_blocks(4), // if, else start, loop, exit
        );
    }
}

// ============================================================================
// INFINITE LOOPS
// ============================================================================

mod infinite_loops {
    use super::*;

    #[test]
    fn loop_with_break() {
        // Based on tmp/10_infinite_loop.ks
        Test::new(
            r#"
            module Main

            func test() -> lang.i64 {
                var i = 0;
                loop {
                    i = lang.i64_add(i, 1);
                    if lang.i64_signed_ge(i, 10) {
                        break
                    }
                }
                i
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.test")
                .returns(MirTy::I64)
                .has_at_least_blocks(3),
        );
    }
}

// ============================================================================
// BREAK AND CONTINUE
// ============================================================================

mod break_continue {
    use super::*;

    #[test]
    fn break_from_while() {
        // Based on tmp/36_break_continue.ks
        Test::new(
            r#"
            module Main

            func findFirst(limit: lang.i64) -> lang.i64 {
                var i = 0;
                while true {
                    if lang.i64_signed_ge(i, limit) {
                        break
                    }
                    i = lang.i64_add(i, 1);
                }
                i
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.findFirst")
                .returns(MirTy::I64)
                .has_at_least_blocks(3),
        );
    }

    #[test]
    fn continue_in_while() {
        // Based on tmp/36_break_continue.ks
        Test::new(
            r#"
            module Main

            func sumOdd(limit: lang.i64) -> lang.i64 {
                var sum = 0;
                var i = 0;
                while lang.i64_signed_lt(i, limit) {
                    i = lang.i64_add(i, 1);
                    if lang.i64_eq(lang.i64_signed_rem(i, 2), 0) {
                        continue
                    }
                    sum = lang.i64_add(sum, i);
                }
                sum
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.sumOdd")
                .returns(MirTy::I64)
                .has_local("sum", MirTy::I64)
                .has_at_least_blocks(4), // entry, loop header, continue block, add block
        );
    }

    #[test]
    fn break_and_continue() {
        // Based on tmp/36_break_continue.ks
        Test::new(
            r#"
            module Main

            func sumUntil(limit: lang.i64) -> lang.i64 {
                var sum = 0;
                var i = 0;
                while true {
                    if lang.i64_signed_ge(i, limit) {
                        break
                    }
                    if lang.i64_eq(lang.i64_signed_rem(i, 2), 0) {
                        i = lang.i64_add(i, 1);
                        continue
                    }
                    sum = lang.i64_add(sum, i);
                    i = lang.i64_add(i, 1);
                }
                sum
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.sumUntil")
                .returns(MirTy::I64)
                .has_at_least_blocks(5),
        );
    }
}

// ============================================================================
// EARLY RETURNS
// ============================================================================

mod early_returns {
    use super::*;

    #[test]
    fn multiple_returns() {
        // Based on tmp/16_early_return.ks
        Test::new(
            r#"
            module Main

            func classify(n: lang.i64) -> lang.i64 {
                if lang.i64_signed_lt(n, 0) {
                    return lang.i64_sub(0, 1)
                }
                if lang.i64_eq(n, 0) {
                    return 0
                }
                if lang.i64_signed_lt(n, 10) {
                    return 1
                }
                if lang.i64_signed_lt(n, 100) {
                    return 2
                }
                3
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.classify")
                .returns(MirTy::I64)
                .has_at_least_blocks(5), // Multiple return paths
        );
    }

    #[test]
    fn return_in_if_else() {
        Test::new(
            r#"
            module Main

            func earlyReturn(x: lang.i64) -> lang.i64 {
                if lang.i64_signed_lt(x, 0) {
                    return 0
                } else {
                    return x
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.earlyReturn")
                .returns(MirTy::I64)
                .any_block(|b| b.terminates_with(TerminatorPattern::Return))
                .any_block(|b| b.terminates_with(TerminatorPattern::Branch)),
        );
    }
}

// ============================================================================
// NESTED CONTROL FLOW
// ============================================================================

mod nested_control_flow {
    use super::*;

    #[test]
    fn nested_if() {
        // Based on tmp/09_nested_control_flow.ks
        Test::new(
            r#"
            module Main

            func nested(x: lang.i64, y: lang.i64) -> lang.i64 {
                if lang.i64_signed_gt(x, 0) {
                    if lang.i64_signed_gt(y, 0) {
                        1
                    } else {
                        2
                    }
                } else {
                    if lang.i64_signed_gt(y, 0) {
                        3
                    } else {
                        4
                    }
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.nested")
                .returns(MirTy::I64)
                .has_at_least_blocks(7), // entry, 2 outer branches, 4 inner branches
        );
    }

    #[test]
    fn loop_in_if() {
        Test::new(
            r#"
            module Main

            func loopInIf(x: lang.i64) -> lang.i64 {
                if lang.i64_signed_gt(x, 0) {
                    var i = 0;
                    while lang.i64_signed_lt(i, x) {
                        i = lang.i64_add(i, 1);
                    }
                    i
                } else {
                    0
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.loopInIf")
                .returns(MirTy::I64)
                .has_at_least_blocks(5),
        );
    }

    #[test]
    fn if_in_loop() {
        Test::new(
            r#"
            module Main

            func ifInLoop(n: lang.i64) -> lang.i64 {
                var count = 0;
                var i = 0;
                while lang.i64_signed_lt(i, n) {
                    if lang.i64_eq(lang.i64_signed_rem(i, 2), 0) {
                        count = lang.i64_add(count, 1);
                    }
                    i = lang.i64_add(i, 1);
                }
                count
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.ifInLoop")
                .returns(MirTy::I64)
                .has_local("count", MirTy::I64)
                .has_at_least_blocks(5),
        );
    }
}

// ============================================================================
// LABELED LOOPS
// ============================================================================

mod labeled_loops {
    use super::*;

    #[test]
    fn labeled_break() {
        // Based on tmp/17_labeled_loops.ks
        Test::new(
            r#"
            module Main

            func test() -> lang.i64 {
                var result = 0;
                'outer: while true {
                    var i = 0;
                    while lang.i64_signed_lt(i, 10) {
                        if lang.i64_eq(i, 5) {
                            break 'outer
                        }
                        i = lang.i64_add(i, 1);
                    }
                    result = lang.i64_add(result, 1);
                }
                result
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.test")
                .returns(MirTy::I64)
                .has_at_least_blocks(5), // Nested loops with labeled break
        );
    }

    #[test]
    fn labeled_continue() {
        Test::new(
            r#"
            module Main

            func test() -> lang.i64 {
                var result = 0;
                var i = 0;
                'outer: while lang.i64_signed_lt(i, 10) {
                    var j = 0;
                    while lang.i64_signed_lt(j, 10) {
                        if lang.i64_eq(j, 5) {
                            i = lang.i64_add(i, 1);
                            continue 'outer
                        }
                        result = lang.i64_add(result, 1);
                        j = lang.i64_add(j, 1);
                    }
                    i = lang.i64_add(i, 1);
                }
                result
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.test")
                .returns(MirTy::I64)
                .has_at_least_blocks(6),
        );
    }
}

// ============================================================================
// COMPLEX CONTROL FLOW
// ============================================================================

mod complex_control_flow {
    use super::*;

    #[test]
    fn multiple_return_paths() {
        // Based on tmp/52_multiple_return_paths.ks
        Test::new(
            r#"
            module Main

            func process(x: lang.i64) -> lang.i64 {
                if lang.i64_signed_lt(x, 0) {
                    return lang.i64_sub(0, x)
                }

                var result = x;
                while lang.i64_signed_gt(result, 100) {
                    result = lang.i64_sub(result, 100);
                    if lang.i64_signed_lt(result, 10) {
                        return lang.i64_mul(result, 2)
                    }
                }

                result
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.process")
                .returns(MirTy::I64)
                .has_local("result", MirTy::I64),
        );
    }
}
