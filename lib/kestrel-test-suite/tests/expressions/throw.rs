//! Tests for throw expressions.
//!
//! The throw expression provides ergonomic error propagation using the `throw` keyword.
//! It desugars to `return R.fromResidual(error)` where R is the function's return type.
//!
//! Syntax: `throw expr`
//!
//! The function's return type must implement `FromResidual[E]` where E is the error type.

use kestrel_test_suite::*;

mod throw_basic {
    use super::*;

    #[test]
    fn basic_throw_in_result_function() {
        Test::new(
            r#"module Test
            struct Error {
                var message: lang.str
            }
            enum Result[T, E] {
                case Ok(T)
                case Err(E)
            }
            extend Result[T, E]: Prelude.FromResidual[E] {
                static func fromResidual(residual: E) -> Result[T, E] {
                    Result.Err(residual)
                }
            }
            func failing() -> Result[lang.i64, Error] {
                throw Error(message: "something went wrong")
            }
        "#,
        )
        .expect(Compiles)
        .expect(Symbol::new("failing").is(SymbolKind::Function));
    }

    #[test]
    fn throw_with_variable() {
        Test::new(
            r#"module Test
            struct Error {
                var message: lang.str
            }
            enum Result[T, E] {
                case Ok(T)
                case Err(E)
            }
            extend Result[T, E]: Prelude.FromResidual[E] {
                static func fromResidual(residual: E) -> Result[T, E] {
                    Result.Err(residual)
                }
            }
            func failing() -> Result[lang.i64, Error] {
                let err = Error(message: "error");
                throw err
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn throw_conditional() {
        Test::new(
            r#"module Test
            struct Error {
                var message: lang.str
            }
            enum Result[T, E] {
                case Ok(T)
                case Err(E)
            }
            extend Result[T, E]: Prelude.FromResidual[E] {
                static func fromResidual(residual: E) -> Result[T, E] {
                    Result.Err(residual)
                }
            }
            func divide(a: lang.i64, b: lang.i64) -> Result[lang.i64, Error] {
                if lang.i64_eq(b, 0) {
                    throw Error(message: "division by zero")
                }
                Result.Ok(lang.i64_signed_div(a, b))
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod throw_with_try {
    use super::*;

    #[test]
    fn throw_with_try_pattern() {
        Test::new(
            r#"module Test
            struct Error {
                var code: lang.i64
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
            func safeDivide(a: lang.i64, b: lang.i64) -> Result[lang.i64, Error] {
                let result = try divide(a, b);
                Result.Ok(result)
            }
            func divide(a: lang.i64, b: lang.i64) -> Result[lang.i64, Error] {
                if lang.i64_eq(b, 0) {
                    throw Error(code: 1)
                }
                Result.Ok(lang.i64_signed_div(a, b))
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod throw_errors {
    use super::*;

    #[test]
    fn throw_outside_function() {
        Test::new(
            r#"module Test
            struct Error {}
            throw Error()
        "#,
        )
        .expect(HasError("Throw"));
    }

    #[test]
    fn throw_without_expression() {
        Test::new(
            r#"module Test
            struct Error {}
            func failing() -> Error {
                throw
            }
        "#,
        )
        .expect(HasError("expected"));
    }
}

mod throw_control_flow {
    use super::*;

    #[test]
    fn throw_in_if_branch() {
        Test::new(
            r#"module Test
            struct Error {}
            enum Result[T, E] {
                case Ok(T)
                case Err(E)
            }
            extend Result[T, E]: Prelude.FromResidual[E] {
                static func fromResidual(residual: E) -> Result[T, E] {
                    Result.Err(residual)
                }
            }
            func test(cond: lang.i1) -> Result[lang.i64, Error] {
                if cond {
                    throw Error()
                }
                Result.Ok(42)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn throw_in_else_branch() {
        Test::new(
            r#"module Test
            struct Error {}
            enum Result[T, E] {
                case Ok(T)
                case Err(E)
            }
            extend Result[T, E]: Prelude.FromResidual[E] {
                static func fromResidual(residual: E) -> Result[T, E] {
                    Result.Err(residual)
                }
            }
            func test(cond: lang.i1) -> Result[lang.i64, Error] {
                if cond {
                    Result.Ok(42)
                } else {
                    throw Error()
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn throw_in_both_branches() {
        Test::new(
            r#"module Test
            struct ErrorA {}
            struct ErrorB {}
            enum Result[T, E] {
                case Ok(T)
                case Err(E)
            }
            extend Result[T, E]: Prelude.FromResidual[E] {
                static func fromResidual(residual: E) -> Result[T, E] {
                    Result.Err(residual)
                }
            }
            func test(cond: lang.i1) -> Result[lang.i64, ErrorA] {
                if cond {
                    throw ErrorA()
                } else {
                    throw ErrorA()
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn throw_in_loop() {
        Test::new(
            r#"module Test
            struct Error {}
            enum Result[T, E] {
                case Ok(T)
                case Err(E)
            }
            extend Result[T, E]: Prelude.FromResidual[E] {
                static func fromResidual(residual: E) -> Result[T, E] {
                    Result.Err(residual)
                }
            }
            func findOrFail(items: lang.i64) -> Result[lang.i64, Error] {
                var i: lang.i64 = 0;
                while lang.i64_signed_lt(i, items) {
                    if lang.i64_eq(i, 5) {
                        throw Error()
                    }
                    i = lang.i64_add(i, 1);
                }
                Result.Ok(i)
            }
        "#,
        )
        .expect(Compiles);
    }
}
