//! Tests for the try operator
//!
//! The try operator provides Rust-style error handling using the `try` keyword.
//! It desugars to a match on the `tryExtract()` method result.
//!
//! Syntax: `try expr`
//!
//! The expression must conform to `Tryable`, which provides:
//! - `tryExtract() -> ControlFlowEnum[Continue, Break]`
//!
//! Where ControlFlowEnum has:
//! - `case Continue(Continue)` - extraction succeeded, unwrap the value
//! - `case Break(Break)` - extraction failed, early return

use kestrel_test_suite::*;

mod basic {
    use super::*;

    #[test]
    fn tryable_protocol_definition() {
        Test::new(
            r#"module Test
            // ControlFlow, Tryable, FromResidual are defined in Prelude
            func test() {
                // Just verify they exist and are accessible
                let _: Prelude.ControlFlow[lang.i64, lang.str] = Prelude.ControlFlow.Continue(42);
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn from_residual_protocol_definition() {
        Test::new(
            r#"module Test
            // FromResidual is defined in Prelude
            struct MyResult {
                var value: lang.i64
            }
            extend MyResult: Prelude.FromResidual[lang.str] {
                static func fromResidual(residual: lang.str) -> MyResult {
                    MyResult(value: 0)
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn option_as_tryable() {
        Test::new(
            r#"module Test
            enum Option[T] {
                case Some(T)
                case None
            }
            struct NoneEarly {}
            extend Option[T]: Prelude.Tryable {
                type Output = T
                type Early = NoneEarly

                func tryExtract() -> Prelude.ControlFlow[T, NoneEarly] {
                    match self {
                        .Some(v) => Prelude.ControlFlow.Continue(v),
                        .None => Prelude.ControlFlow.Break(NoneEarly())
                    }
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn result_as_tryable() {
        Test::new(
            r#"module Test
            enum Result[T, E] {
                case Ok(T)
                case Err(E)
            }
            extend Result[T, E]: Prelude.Tryable {
                type Output = T
                type Early = E

                func tryExtract() -> Prelude.ControlFlow[T, E] {
                    match self {
                        .Ok(v) => Prelude.ControlFlow.Continue(v),
                        .Err(e) => Prelude.ControlFlow.Break(e)
                    }
                }
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod try_expression {
    use super::*;

    #[test]
    fn try_on_option() {
        Test::new(
            r#"module Test
            enum Option[T] {
                case Some(T)
                case None
            }
            struct NoneEarly {}
            extend Option[T]: Prelude.Tryable {
                type Output = T
                type Early = NoneEarly

                func tryExtract() -> Prelude.ControlFlow[T, NoneEarly] {
                    match self {
                        .Some(v) => Prelude.ControlFlow.Continue(v),
                        .None => Prelude.ControlFlow.Break(NoneEarly())
                    }
                }
            }
            extend Option[T]: Prelude.FromResidual[NoneEarly] {
                static func fromResidual(residual: NoneEarly) -> Option[T] {
                    Option.None
                }
            }
            func maybeDouble(opt: Option[lang.i64]) -> Option[lang.i64] {
                let value = try opt;
                Option.Some(lang.i64_mul(value, 2))
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn try_on_result() {
        Test::new(
            r#"module Test
            struct Error {
                var message: lang.str
            }
            enum Result[T, E] {
                case Ok(T)
                case Err(E)
            }
            extend Result[T, E]: Prelude.Tryable {
                type Output = T
                type Early = E

                func tryExtract() -> Prelude.ControlFlow[T, E] {
                    match self {
                        .Ok(v) => Prelude.ControlFlow.Continue(v),
                        .Err(e) => Prelude.ControlFlow.Break(e)
                    }
                }
            }
            extend Result[T, E]: Prelude.FromResidual[E] {
                static func fromResidual(residual: E) -> Result[T, E] {
                    Result.Err(residual)
                }
            }
            func divide(a: lang.i64, b: lang.i64) -> Result[lang.i64, Error] {
                if lang.i64_eq(b, 0) {
                    return Result.Err(Error(message: "division by zero"))
                }
                Result.Ok(lang.i64_signed_div(a, b))
            }
            func compute(a: lang.i64, b: lang.i64) -> Result[lang.i64, Error] {
                let quotient = try divide(a, b);
                Result.Ok(lang.i64_add(quotient, 1))
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn multiple_try_in_function() {
        Test::new(
            r#"module Test
            enum Option[T] {
                case Some(T)
                case None
            }
            struct NoneEarly {}
            extend Option[T]: Prelude.Tryable {
                type Output = T
                type Early = NoneEarly

                func tryExtract() -> Prelude.ControlFlow[T, NoneEarly] {
                    match self {
                        .Some(v) => Prelude.ControlFlow.Continue(v),
                        .None => Prelude.ControlFlow.Break(NoneEarly())
                    }
                }
            }
            extend Option[T]: Prelude.FromResidual[NoneEarly] {
                static func fromResidual(residual: NoneEarly) -> Option[T] {
                    Option.None
                }
            }
            func add(a: Option[lang.i64], b: Option[lang.i64]) -> Option[lang.i64] {
                let x = try a;
                let y = try b;
                Option.Some(lang.i64_add(x, y))
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn try_in_expression_context() {
        Test::new(
            r#"module Test
            enum Option[T] {
                case Some(T)
                case None
            }
            struct NoneEarly {}
            extend Option[T]: Prelude.Tryable {
                type Output = T
                type Early = NoneEarly

                func tryExtract() -> Prelude.ControlFlow[T, NoneEarly] {
                    match self {
                        .Some(v) => Prelude.ControlFlow.Continue(v),
                        .None => Prelude.ControlFlow.Break(NoneEarly())
                    }
                }
            }
            extend Option[T]: Prelude.FromResidual[NoneEarly] {
                static func fromResidual(residual: NoneEarly) -> Option[T] {
                    Option.None
                }
            }
            func doubleAndAdd(opt: Option[lang.i64], addend: lang.i64) -> Option[lang.i64] {
                Option.Some(lang.i64_add(lang.i64_mul(try opt, 2), addend))
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod control_flow {
    use super::*;

    #[test]
    fn control_flow_structure() {
        Test::new(
            r#"module Test
            func test() {
                let cont: Prelude.ControlFlow[lang.i64, lang.str] = Prelude.ControlFlow.Continue(42);
                let brk: Prelude.ControlFlow[lang.i64, lang.str] = Prelude.ControlFlow.Break("error");
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn match_on_control_flow() {
        Test::new(
            r#"module Test
            func extract(cf: Prelude.ControlFlow[lang.i64, lang.str]) -> lang.i64 {
                match cf {
                    .Continue(value) => value,
                    .Break(_msg) => 0
                }
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod errors {
    use super::*;

    #[test]
    fn try_on_non_tryable_type() {
        Test::new(
            r#"module Test
            struct NotTryable {
                var value: lang.i64
            }
            func test() -> lang.i64 {
                let n = NotTryable(value: 42);
                try n
            }
        "#,
        )
        .expect(HasError("Tryable"));
    }

    #[test]
    fn try_return_type_mismatch() {
        Test::new(
            r#"module Test
            enum Option[T] {
                case Some(T)
                case None
            }
            struct NoneEarly {}
            extend Option[T]: Prelude.Tryable {
                type Output = T
                type Early = NoneEarly

                func tryExtract() -> Prelude.ControlFlow[T, NoneEarly] {
                    match self {
                        .Some(v) => Prelude.ControlFlow.Continue(v),
                        .None => Prelude.ControlFlow.Break(NoneEarly())
                    }
                }
            }
            // Missing FromResidual conformance, so try can't convert the early value
            func maybeValue(opt: Option[lang.i64]) -> lang.i64 {
                try opt
            }
        "#,
        )
        .expect(HasError("fromResidual"));
    }
}

mod precedence {
    use super::*;

    #[test]
    fn try_high_precedence() {
        // try should bind tighter than binary operators
        // try a + b should be (try a) + b, not try (a + b)
        Test::new(
            r#"module Test
            enum Option[T] {
                case Some(T)
                case None
            }
            struct NoneEarly {}
            extend Option[T]: Prelude.Tryable {
                type Output = T
                type Early = NoneEarly

                func tryExtract() -> Prelude.ControlFlow[T, NoneEarly] {
                    match self {
                        .Some(v) => Prelude.ControlFlow.Continue(v),
                        .None => Prelude.ControlFlow.Break(NoneEarly())
                    }
                }
            }
            extend Option[T]: Prelude.FromResidual[NoneEarly] {
                static func fromResidual(residual: NoneEarly) -> Option[T] {
                    Option.None
                }
            }
            func addToOption(opt: Option[lang.i64], value: lang.i64) -> Option[lang.i64] {
                Option.Some(lang.i64_add(try opt, value))
            }
        "#,
        )
        .expect(Compiles);
    }
}

// ============================================================================
// Execution tests - these compile, link, and run to verify correctness
// ============================================================================

mod execution {
    use super::*;

    /// Helper: Result type with Tryable and FromResidual implementations
    const RESULT_PRELUDE: &str = r#"
        enum Result[T, E] {
            case Ok(T)
            case Err(E)
        }
        extend Result[T, E]: Prelude.Tryable {
            type Output = T
            type Early = E

            func tryExtract() -> Prelude.ControlFlow[T, E] {
                match self {
                    .Ok(v) => Prelude.ControlFlow.Continue(v),
                    .Err(e) => Prelude.ControlFlow.Break(e)
                }
            }
        }
        extend Result[T, E]: Prelude.FromResidual[E] {
            static func fromResidual(residual: E) -> Result[T, E] {
                Result.Err(residual)
            }
        }
    "#;

    #[test]
    fn try_result_success_path() {
        // When Result is Ok, try should extract the value
        Test::new(&format!(
            r#"module Test
            {RESULT_PRELUDE}

            func compute(r: Result[lang.i64, lang.i64]) -> Result[lang.i64, lang.i64] {{
                let value = try r;
                Result.Ok(lang.i64_mul(value, 2))
            }}

            func main() -> lang.i64 {{
                let result = compute(Result.Ok(21));
                match result {{
                    .Ok(v) => v,  // Should be 42
                    .Err(_) => 0
                }}
            }}
        "#
        ))
        .expect(ExitCode(42));
    }

    #[test]
    fn try_result_failure_path() {
        // When Result is Err, try should propagate the error
        Test::new(&format!(
            r#"module Test
            {RESULT_PRELUDE}

            func compute(r: Result[lang.i64, lang.i64]) -> Result[lang.i64, lang.i64] {{
                let value = try r;
                Result.Ok(lang.i64_mul(value, 100))  // Should not execute
            }}

            func main() -> lang.i64 {{
                let result = compute(Result.Err(77));
                match result {{
                    .Ok(_) => 0,
                    .Err(e) => e  // Should be 77
                }}
            }}
        "#
        ))
        .expect(ExitCode(77));
    }

    #[test]
    fn multiple_try_all_succeed() {
        // Multiple try expressions, all succeed
        Test::new(&format!(
            r#"module Test
            {RESULT_PRELUDE}

            func addThree(a: Result[lang.i64, lang.i64], b: Result[lang.i64, lang.i64], c: Result[lang.i64, lang.i64]) -> Result[lang.i64, lang.i64] {{
                let x = try a;
                let y = try b;
                let z = try c;
                Result.Ok(lang.i64_add(lang.i64_add(x, y), z))
            }}

            func main() -> lang.i64 {{
                let result = addThree(Result.Ok(10), Result.Ok(20), Result.Ok(12));
                match result {{
                    .Ok(v) => v,  // Should be 42
                    .Err(_) => 0
                }}
            }}
        "#
        ))
        .expect(ExitCode(42));
    }

    #[test]
    fn multiple_try_first_fails() {
        // Multiple try expressions, first one fails
        Test::new(&format!(
            r#"module Test
            {RESULT_PRELUDE}

            func addThree(a: Result[lang.i64, lang.i64], b: Result[lang.i64, lang.i64], c: Result[lang.i64, lang.i64]) -> Result[lang.i64, lang.i64] {{
                let x = try a;  // This fails
                let y = try b;
                let z = try c;
                Result.Ok(lang.i64_add(lang.i64_add(x, y), z))
            }}

            func main() -> lang.i64 {{
                let result = addThree(Result.Err(88), Result.Ok(20), Result.Ok(12));
                match result {{
                    .Ok(_) => 0,
                    .Err(e) => e  // Should be 88
                }}
            }}
        "#
        ))
        .expect(ExitCode(88));
    }

    #[test]
    fn multiple_try_middle_fails() {
        // Multiple try expressions, middle one fails
        Test::new(&format!(
            r#"module Test
            {RESULT_PRELUDE}

            func addThree(a: Result[lang.i64, lang.i64], b: Result[lang.i64, lang.i64], c: Result[lang.i64, lang.i64]) -> Result[lang.i64, lang.i64] {{
                let x = try a;
                let y = try b;  // This fails
                let z = try c;
                Result.Ok(lang.i64_add(lang.i64_add(x, y), z))
            }}

            func main() -> lang.i64 {{
                let result = addThree(Result.Ok(10), Result.Err(77), Result.Ok(12));
                match result {{
                    .Ok(_) => 0,
                    .Err(e) => e  // Should be 77
                }}
            }}
        "#
        ))
        .expect(ExitCode(77));
    }

    #[test]
    fn try_in_nested_expression() {
        // Try in a nested expression context
        Test::new(&format!(
            r#"module Test
            {RESULT_PRELUDE}

            func compute(r: Result[lang.i64, lang.i64]) -> Result[lang.i64, lang.i64] {{
                Result.Ok(lang.i64_mul(lang.i64_add(try r, 1), 2))
            }}

            func main() -> lang.i64 {{
                let result = compute(Result.Ok(20));
                match result {{
                    .Ok(v) => v,  // (20 + 1) * 2 = 42
                    .Err(_) => 0
                }}
            }}
        "#
        ))
        .expect(ExitCode(42));
    }

    #[test]
    fn try_with_different_error_types() {
        // Try with Result where Ok and Err have different types
        Test::new(&format!(
            r#"module Test
            {RESULT_PRELUDE}

            struct MyError {{
                var code: lang.i64
            }}

            func compute(r: Result[lang.i64, MyError]) -> Result[lang.i64, MyError] {{
                let value = try r;
                Result.Ok(lang.i64_add(value, 10))
            }}

            func main() -> lang.i64 {{
                let result = compute(Result.Err(MyError(code: 55)));
                match result {{
                    .Ok(_) => 0,
                    .Err(e) => e.code  // Should be 55
                }}
            }}
        "#
        ))
        .expect(ExitCode(55));
    }

    #[test]
    fn try_chained_function_calls() {
        // Try in chained function calls - both succeed
        Test::new(&format!(
            r#"module Test
            {RESULT_PRELUDE}

            func step1(x: lang.i64) -> Result[lang.i64, lang.i64] {{
                Result.Ok(lang.i64_add(x, 10))
            }}

            func step2(x: lang.i64) -> Result[lang.i64, lang.i64] {{
                Result.Ok(lang.i64_mul(x, 2))
            }}

            func pipeline(x: lang.i64) -> Result[lang.i64, lang.i64] {{
                let a = try step1(x);
                let b = try step2(a);
                Result.Ok(b)
            }}

            func main() -> lang.i64 {{
                let result = pipeline(16);
                match result {{
                    .Ok(v) => v,  // (16 + 10) * 2 = 52
                    .Err(_) => 0
                }}
            }}
        "#
        ))
        .expect(ExitCode(52));
    }

    #[test]
    fn try_early_return_in_chain() {
        // Early return when first step fails
        Test::new(&format!(
            r#"module Test
            {RESULT_PRELUDE}

            func step1_fail(_x: lang.i64) -> Result[lang.i64, lang.i64] {{
                Result.Err(99)
            }}

            func step2(x: lang.i64) -> Result[lang.i64, lang.i64] {{
                Result.Ok(lang.i64_mul(x, 2))
            }}

            func pipeline(x: lang.i64) -> Result[lang.i64, lang.i64] {{
                let a = try step1_fail(x);  // Returns Err(99)
                let b = try step2(a);       // Never reached
                Result.Ok(b)
            }}

            func main() -> lang.i64 {{
                let result = pipeline(15);
                match result {{
                    .Ok(_) => 0,
                    .Err(e) => e  // Should be 99
                }}
            }}
        "#
        ))
        .expect(ExitCode(99));
    }
}

// ============================================================================
// Deinit tests - verify try works correctly with types that have destructors
// These tests use the stdlib's Result type which is already Tryable
// ============================================================================

mod deinit_interaction {
    use super::*;

    #[test]
    fn try_compiles_with_copyable_types() {
        // Basic test that try compiles with copyable types
        // More complex deinit interaction tests require stdlib support
        Test::new(
            r#"module Test
            enum Result[T, E] {
                case Ok(T)
                case Err(E)
            }
            extend Result[T, E]: Prelude.Tryable {
                type Output = T
                type Early = E

                func tryExtract() -> Prelude.ControlFlow[T, E] {
                    match self {
                        .Ok(v) => Prelude.ControlFlow.Continue(v),
                        .Err(e) => Prelude.ControlFlow.Break(e)
                    }
                }
            }
            extend Result[T, E]: Prelude.FromResidual[E] {
                static func fromResidual(residual: E) -> Result[T, E] {
                    Result.Err(residual)
                }
            }

            struct Data {
                var x: lang.i64
                var y: lang.i64
            }

            func compute(r: Result[Data, lang.i64]) -> Result[lang.i64, lang.i64] {
                let data = try r;
                Result.Ok(lang.i64_add(data.x, data.y))
            }

            func main() -> lang.i64 {
                let result = compute(Result.Ok(Data(x: 20, y: 22)));
                match result {
                    .Ok(v) => v,  // 20 + 22 = 42
                    .Err(_) => 0
                }
            }
        "#,
        )
        .expect(ExitCode(42));
    }

    #[test]
    fn try_with_struct_error_type() {
        // Test try with a struct as the error type (common pattern)
        Test::new(
            r#"module Test
            enum Result[T, E] {
                case Ok(T)
                case Err(E)
            }
            extend Result[T, E]: Prelude.Tryable {
                type Output = T
                type Early = E

                func tryExtract() -> Prelude.ControlFlow[T, E] {
                    match self {
                        .Ok(v) => Prelude.ControlFlow.Continue(v),
                        .Err(e) => Prelude.ControlFlow.Break(e)
                    }
                }
            }
            extend Result[T, E]: Prelude.FromResidual[E] {
                static func fromResidual(residual: E) -> Result[T, E] {
                    Result.Err(residual)
                }
            }

            struct MyError {
                var code: lang.i64
            }

            func failWith(code: lang.i64) -> Result[lang.i64, MyError] {
                Result.Err(MyError(code: code))
            }

            func compute() -> Result[lang.i64, MyError] {
                let _ = try failWith(55);
                Result.Ok(0)
            }

            func main() -> lang.i64 {
                let result = compute();
                match result {
                    .Ok(_) => 0,
                    .Err(e) => e.code  // Should be 55
                }
            }
        "#,
        )
        .expect(ExitCode(55));
    }
}

// ============================================================================
// Witness method tests - verify static extension methods work correctly
// ============================================================================

mod witness_methods {
    use super::*;

    #[test]
    fn from_residual_called_on_error() {
        // Verify that fromResidual is correctly called as a static method
        // This tests the witness method mangling fix
        // The error value should be preserved through fromResidual
        Test::new(
            r#"module Test
            enum Result[T, E] {
                case Ok(T)
                case Err(E)
            }
            extend Result[T, E]: Prelude.Tryable {
                type Output = T
                type Early = E

                func tryExtract() -> Prelude.ControlFlow[T, E] {
                    match self {
                        .Ok(v) => Prelude.ControlFlow.Continue(v),
                        .Err(e) => Prelude.ControlFlow.Break(e)
                    }
                }
            }
            extend Result[T, E]: Prelude.FromResidual[E] {
                static func fromResidual(residual: E) -> Result[T, E] {
                    // fromResidual wraps the error in Err
                    Result.Err(residual)
                }
            }

            func compute(r: Result[lang.i64, lang.i64]) -> Result[lang.i64, lang.i64] {
                let v = try r;  // If Err, calls fromResidual
                Result.Ok(lang.i64_add(v, 100))
            }

            func main() -> lang.i64 {
                let result = compute(Result.Err(42));
                match result {
                    .Ok(_) => 0,
                    .Err(e) => e  // Should be 42 (preserved through fromResidual)
                }
            }
        "#,
        )
        .expect(ExitCode(42));
    }

    #[test]
    fn from_residual_with_generic_error_type() {
        // Test fromResidual with a generic error type to verify type parameter handling
        Test::new(
            r#"module Test
            struct Error[T] {
                var data: T
            }

            enum Result[V, E] {
                case Ok(V)
                case Err(E)
            }
            extend Result[V, E]: Prelude.Tryable {
                type Output = V
                type Early = E

                func tryExtract() -> Prelude.ControlFlow[V, E] {
                    match self {
                        .Ok(v) => Prelude.ControlFlow.Continue(v),
                        .Err(e) => Prelude.ControlFlow.Break(e)
                    }
                }
            }
            extend Result[V, E]: Prelude.FromResidual[E] {
                static func fromResidual(residual: E) -> Result[V, E] {
                    Result.Err(residual)
                }
            }

            func compute(r: Result[lang.i64, Error[lang.i64]]) -> Result[lang.i64, Error[lang.i64]] {
                let v = try r;
                Result.Ok(lang.i64_add(v, 10))
            }

            func main() -> lang.i64 {
                let result = compute(Result.Err(Error(data: 32)));
                match result {
                    .Ok(_) => 0,
                    .Err(e) => e.data  // Should be 32
                }
            }
        "#,
        )
        .expect(ExitCode(32));
    }

    #[test]
    fn different_result_types_in_program() {
        // Test multiple Result types in the same program
        // This ensures witness resolution works for multiple instantiations
        Test::new(
            r#"module Test
            enum Result[T, E] {
                case Ok(T)
                case Err(E)
            }
            extend Result[T, E]: Prelude.Tryable {
                type Output = T
                type Early = E

                func tryExtract() -> Prelude.ControlFlow[T, E] {
                    match self {
                        .Ok(v) => Prelude.ControlFlow.Continue(v),
                        .Err(e) => Prelude.ControlFlow.Break(e)
                    }
                }
            }
            extend Result[T, E]: Prelude.FromResidual[E] {
                static func fromResidual(residual: E) -> Result[T, E] {
                    Result.Err(residual)
                }
            }

            struct ErrorA { var code: lang.i64 }
            struct ErrorB { var code: lang.i64 }

            func computeA(r: Result[lang.i64, ErrorA]) -> Result[lang.i64, ErrorA] {
                let v = try r;
                Result.Ok(lang.i64_add(v, 10))
            }

            func computeB(r: Result[lang.i64, ErrorB]) -> Result[lang.i64, ErrorB] {
                let v = try r;
                Result.Ok(lang.i64_mul(v, 2))
            }

            func main() -> lang.i64 {
                // Test with ErrorA - success path
                let resultA = computeA(Result.Ok(16));
                let a = match resultA {
                    .Ok(v) => v,  // 16 + 10 = 26
                    .Err(_) => 0
                };

                // Test with ErrorB - success path
                let resultB = computeB(Result.Ok(8));
                let b = match resultB {
                    .Ok(v) => v,  // 8 * 2 = 16
                    .Err(_) => 0
                };

                lang.i64_add(a, b)  // 26 + 16 = 42
            }
        "#,
        )
        .expect(ExitCode(42));
    }
}
