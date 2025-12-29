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

            func countdown(n: Int) -> Int {
                var i = n;
                while i > 0 {
                    i = i - 1;
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

            func factorial(n: Int) -> Int {
                var result = 1;
                var i = n;
                while i > 1 {
                    result = result * i;
                    i = i - 1;
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

            func fibonacci(n: Int) -> Int {
                if n <= 1 {
                    n
                } else {
                    var a = 0;
                    var b = 1;
                    var i = 2;
                    while i <= n {
                        let temp = a + b;
                        a = b;
                        b = temp;
                        i = i + 1;
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

            func test() -> Int {
                var i = 0;
                loop {
                    i = i + 1;
                    if i >= 10 {
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

            func findFirst(limit: Int) -> Int {
                var i = 0;
                while true {
                    if i >= limit {
                        break
                    }
                    i = i + 1;
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

            func sumOdd(limit: Int) -> Int {
                var sum = 0;
                var i = 0;
                while i < limit {
                    i = i + 1;
                    if i % 2 == 0 {
                        continue
                    }
                    sum = sum + i;
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

            func sumUntil(limit: Int) -> Int {
                var sum = 0;
                var i = 0;
                while true {
                    if i >= limit {
                        break
                    }
                    if i % 2 == 0 {
                        i = i + 1;
                        continue
                    }
                    sum = sum + i;
                    i = i + 1;
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

            func classify(n: Int) -> Int {
                if n < 0 {
                    return 0 - 1
                }
                if n == 0 {
                    return 0
                }
                if n < 10 {
                    return 1
                }
                if n < 100 {
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

            func earlyReturn(x: Int) -> Int {
                if x < 0 {
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

            func nested(x: Int, y: Int) -> Int {
                if x > 0 {
                    if y > 0 {
                        1
                    } else {
                        2
                    }
                } else {
                    if y > 0 {
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

            func loopInIf(x: Int) -> Int {
                if x > 0 {
                    var i = 0;
                    while i < x {
                        i = i + 1;
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

            func ifInLoop(n: Int) -> Int {
                var count = 0;
                var i = 0;
                while i < n {
                    if i % 2 == 0 {
                        count = count + 1;
                    }
                    i = i + 1;
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

            func test() -> Int {
                var result = 0;
                'outer: while true {
                    var i = 0;
                    while i < 10 {
                        if i == 5 {
                            break 'outer
                        }
                        i = i + 1;
                    }
                    result = result + 1;
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

            func test() -> Int {
                var result = 0;
                var i = 0;
                'outer: while i < 10 {
                    var j = 0;
                    while j < 10 {
                        if j == 5 {
                            i = i + 1;
                            continue 'outer
                        }
                        result = result + 1;
                        j = j + 1;
                    }
                    i = i + 1;
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

            func process(x: Int) -> Int {
                if x < 0 {
                    return 0 - x
                }
                
                var result = x;
                while result > 100 {
                    result = result - 100;
                    if result < 10 {
                        return result * 2
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
