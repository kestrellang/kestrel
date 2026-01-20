//! Closure MIR tests.
//!
//! Tests for closure lowering including:
//! - Non-capturing closures
//! - Capturing closures with environment structs
//! - Multi-statement closures
//! - Nested closures
//! - Closure invocation

use kestrel_test_suite::mir::*;
use kestrel_test_suite::*;

// ============================================================================
// NON-CAPTURING CLOSURES
// ============================================================================

mod non_capturing {
    use super::*;

    #[test]
    fn closure_no_params_returns_constant() {
        // Simplest closure: no params, returns constant
        Test::new(
            r#"
            module Test

            func test() -> () -> lang.i64 {
                { 42 }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_closure("Test.test", 0)
                .returns(MirTy::I64)
                .is_non_capturing(),
        );
    }

    #[test]
    fn closure_with_explicit_params() {
        // Closure with explicit parameters
        Test::new(
            r#"
            module Test

            func test() -> (lang.i64, lang.i64) -> lang.i64 {
                { (x, y) in lang.i64_add(x, y) }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_closure("Test.test", 0)
                .returns(MirTy::I64)
                .has_param("x", MirTy::I64)
                .has_param("y", MirTy::I64)
                .is_non_capturing(),
        );
    }

    #[test]
    fn closure_with_implicit_it_param() {
        // Closure with implicit `it` parameter
        Test::new(
            r#"
            module Test

            func test() -> (lang.i64) -> lang.i64 {
                { lang.i64_mul(it, 2) }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_closure("Test.test", 0)
                .returns(MirTy::I64)
                .has_param("it", MirTy::I64)
                .is_non_capturing(),
        );
    }

    #[test]
    fn closure_empty_params_returns_unit() {
        // Closure with () -> ()
        Test::new(
            r#"
            module Test

            func test() -> () -> () {
                { () }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_closure("Test.test", 0).returns(MirTy::Unit));
    }
}

// ============================================================================
// CAPTURING CLOSURES
// ============================================================================

mod capturing {
    use super::*;

    #[test]
    fn single_capture_from_parameter() {
        // Closure captures function parameter
        Test::new(
            r#"
            module Test

            func test(n: lang.i64) -> () -> lang.i64 {
                { lang.i64_add(n, 1) }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_closure("Test.test", 0).has_captures(1));
    }

    #[test]
    fn single_capture_from_let() {
        // Closure captures local let binding
        Test::new(
            r#"
            module Test

            func test() -> () -> lang.i64 {
                let x = 42;
                { x }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_closure("Test.test", 0).has_captures(1));
    }

    #[test]
    fn multiple_captures() {
        // Closure captures multiple variables
        Test::new(
            r#"
            module Test

            func test() -> () -> lang.i64 {
                let a = 1;
                let b = 2;
                let c = 3;
                { lang.i64_add(lang.i64_add(a, b), c) }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_closure("Test.test", 0).has_captures(1));
    }

    #[test]
    fn capture_with_params() {
        // Closure has both captures and parameters
        Test::new(
            r#"
            module Test

            func test(multiplier: lang.i64) -> (lang.i64) -> lang.i64 {
                { lang.i64_mul(it, multiplier) }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_closure("Test.test", 0)
                .has_param("it", MirTy::I64)
                .has_captures(1),
        );
    }
}

// ============================================================================
// MULTI-STATEMENT CLOSURES
// ============================================================================

mod multi_statement {
    use super::*;

    #[test]
    fn closure_with_let_binding() {
        Test::new(
            r#"
            module Test

            func test() -> (lang.i64) -> lang.i64 {
                { (x) in
                    let y = lang.i64_mul(x, 2);
                    lang.i64_add(y, 1)
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_closure("Test.test", 0)
                .returns(MirTy::I64)
                .has_local("y", MirTy::I64),
        );
    }

    #[test]
    fn closure_with_control_flow() {
        Test::new(
            r#"
            module Test

            func test() -> (lang.i64) -> lang.i64 {
                { (x) in
                    if lang.i64_signed_gt(x, 0) {
                        x
                    } else {
                        lang.i64_sub(0, x)
                    }
                }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_closure("Test.test", 0)
                .returns(MirTy::I64)
                .has_at_least_blocks(3) // entry, then, else (maybe join)
                .any_block(|b| b.terminates_with(TerminatorPattern::Branch)),
        );
    }
}

// ============================================================================
// MULTIPLE CLOSURES
// ============================================================================

mod multiple_closures {
    use super::*;

    #[test]
    fn two_closures_in_same_function() {
        Test::new(
            r#"
            module Test

            func test() -> lang.i64 {
                let f: () -> lang.i64 = { 1 };
                let g: () -> lang.i64 = { 2 };
                lang.i64_add(f(), g())
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_closure("Test.test", 0).returns(MirTy::I64))
        .expect(Mir::mir_closure("Test.test", 1).returns(MirTy::I64));
    }

    #[test]
    fn closures_in_different_functions() {
        Test::new(
            r#"
            module Test

            func foo() -> () -> lang.i64 {
                { 1 }
            }

            func bar() -> () -> lang.i64 {
                { 2 }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_closure("Test.foo", 0).returns(MirTy::I64))
        .expect(Mir::mir_closure("Test.bar", 0).returns(MirTy::I64));
    }
}

// ============================================================================
// NESTED CLOSURES
// ============================================================================

mod nested {
    use super::*;

    #[test]
    fn closure_returning_closure() {
        Test::new(
            r#"
            module Test

            func test() -> (lang.i64) -> (lang.i64) -> lang.i64 {
                { (x) in { (y) in lang.i64_add(x, y) } }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        // Outer closure
        .expect(Mir::mir_closure("Test.test", 0).has_param("x", MirTy::I64));
        // Inner closure would be at Test."test.closure.0".closure.0
    }

    #[test]
    fn inner_closure_captures_outer_param() {
        Test::new(
            r#"
            module Test

            func test() -> lang.i64 {
                let f: (lang.i64) -> (lang.i64) -> lang.i64 = { (x) in { (y) in lang.i64_add(x, y) } };
                let add10 = f(10);
                add10(5)
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_closure("Test.test", 0).has_param("x", MirTy::I64));
    }
}

// ============================================================================
// CLOSURE INVOCATION
// ============================================================================

mod invocation {
    use super::*;

    #[test]
    fn immediately_invoked_closure() {
        Test::new(
            r#"
            module Test

            func test() -> lang.i64 {
                { 42 }()
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_closure("Test.test", 0).returns(MirTy::I64))
        .expect(Mir::mir_function("Test.test").calls_escaping());
    }

    #[test]
    fn closure_stored_and_called() {
        Test::new(
            r#"
            module Test

            func test() -> lang.i64 {
                let f: (lang.i64) -> lang.i64 = { lang.i64_mul(it, 2) };
                f(21)
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_closure("Test.test", 0).returns(MirTy::I64))
        .expect(Mir::mir_function("Test.test").calls_escaping());
    }
}

// ============================================================================
// CLOSURE AS PARAMETER
// ============================================================================

mod closure_as_parameter {
    use super::*;

    #[test]
    fn function_taking_closure() {
        Test::new(
            r#"
            module Main

            func apply(f: (lang.i64) -> lang.i64, x: lang.i64) -> lang.i64 {
                f(x)
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.apply")
                .returns(MirTy::I64)
                .has_param_count(2)
                .calls_escaping(),
        );
    }

    #[test]
    fn compose_closures() {
        Test::new(
            r#"
            module Main

            func compose(f: (lang.i64) -> lang.i64, g: (lang.i64) -> lang.i64, x: lang.i64) -> lang.i64 {
                f(g(x))
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.compose")
                .returns(MirTy::I64)
                .has_param_count(3)
                .calls_escaping(),
        );
    }

    #[test]
    fn main_with_closures() {
        // Based on tmp/06_closures.ks
        Test::new(
            r#"
            module Main

            func apply(f: (lang.i64) -> lang.i64, x: lang.i64) -> lang.i64 {
                f(x)
            }

            func main() -> lang.i64 {
                let double = { (x: lang.i64) in lang.i64_mul(x, 2) };
                let addOne = { (x: lang.i64) in lang.i64_add(x, 1) };

                let a = apply(double, 5);
                let b = apply(addOne, 10);

                lang.i64_add(a, b)
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(Mir::mir_function("Main.main").calls("Main.apply"))
        .expect(
            Mir::mir_closure("Main.main", 0)
                .any_block(|b| b.has_statement(StatementPattern::BinOp(BinOp::MulSigned))),
        )
        .expect(
            Mir::mir_closure("Main.main", 1)
                .any_block(|b| b.has_statement(StatementPattern::BinOp(BinOp::AddSigned))),
        );
    }
}

// ============================================================================
// CLOSURE CAPTURE (makeAdder pattern)
// ============================================================================

mod make_adder {
    use super::*;

    #[test]
    fn make_adder_returns_closure() {
        // Based on tmp/13_closure_capture.ks
        // Note: Function parameters default to borrow mode
        Test::new(
            r#"
            module Main

            func makeAdder(n: lang.i64) -> (lang.i64) -> lang.i64 {
                { (x: lang.i64) in lang.i64_add(x, n) }
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.makeAdder")
                .returns(MirTy::func_thick(vec![MirTy::I64], MirTy::I64))
                .has_param("n", MirTy::ref_(MirTy::I64)),
        )
        .expect(
            Mir::mir_closure("Main.makeAdder", 0)
                .returns(MirTy::I64)
                .has_param("x", MirTy::I64)
                .has_captures(1),
        );
    }

    #[test]
    fn make_adder_usage() {
        Test::new(
            r#"
            module Main

            func makeAdder(n: lang.i64) -> (lang.i64) -> lang.i64 {
                { (x: lang.i64) in lang.i64_add(x, n) }
            }

            func main() -> lang.i64 {
                let add5 = makeAdder(5);
                let add10 = makeAdder(10);

                lang.i64_add(add5(3), add10(3))
            }
        "#,
        )
        .expect(Compiles)
        .expect(Mir::compiles())
        .expect(
            Mir::mir_function("Main.main")
                .calls("Main.makeAdder")
                .calls_escaping(),
        );
    }
}
