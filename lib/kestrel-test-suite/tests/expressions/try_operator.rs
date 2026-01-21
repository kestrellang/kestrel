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
    #[ignore]
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
    #[ignore]
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
    #[ignore]
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
                        case .Some(let v) => Prelude.ControlFlow.Continue(v)
                        case .None => Prelude.ControlFlow.Break(NoneEarly())
                    }
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    #[ignore]
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
                        case .Ok(let v) => Prelude.ControlFlow.Continue(v)
                        case .Err(let e) => Prelude.ControlFlow.Break(e)
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
    #[ignore]
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
                        case .Some(let v) => Prelude.ControlFlow.Continue(v)
                        case .None => Prelude.ControlFlow.Break(NoneEarly())
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
    #[ignore]
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
                        case .Ok(let v) => Prelude.ControlFlow.Continue(v)
                        case .Err(let e) => Prelude.ControlFlow.Break(e)
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
    #[ignore]
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
                        case .Some(let v) => Prelude.ControlFlow.Continue(v)
                        case .None => Prelude.ControlFlow.Break(NoneEarly())
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
    #[ignore]
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
                        case .Some(let v) => Prelude.ControlFlow.Continue(v)
                        case .None => Prelude.ControlFlow.Break(NoneEarly())
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
    #[ignore]
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
    #[ignore]
    fn match_on_control_flow() {
        Test::new(
            r#"module Test
            func extract(cf: Prelude.ControlFlow[lang.i64, lang.str]) -> lang.i64 {
                match cf {
                    case .Continue(let value) => value
                    case .Break(let _msg) => 0
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
    #[ignore]
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
    #[ignore]
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
                        case .Some(let v) => Prelude.ControlFlow.Continue(v)
                        case .None => Prelude.ControlFlow.Break(NoneEarly())
                    }
                }
            }
            // Missing FromResidual conformance, so try can't convert the early value
            func maybeValue(opt: Option[lang.i64]) -> lang.i64 {
                try opt
            }
        "#,
        )
        .expect(HasError("FromResidual"));
    }
}

mod precedence {
    use super::*;

    #[test]
    #[ignore]
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
                        case .Some(let v) => Prelude.ControlFlow.Continue(v)
                        case .None => Prelude.ControlFlow.Break(NoneEarly())
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
