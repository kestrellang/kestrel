// test: diagnostics
// stdlib: false

module Test
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

func step1_fail(_x: lang.i64) -> Result[lang.i64, lang.i64] {
    Result.Err(99)
}

func step2(x: lang.i64) -> Result[lang.i64, lang.i64] {
    Result.Ok(lang.i64_mul(x, 2))
}

func pipeline(x: lang.i64) -> Result[lang.i64, lang.i64] {
    let a = try step1_fail(x);
    let b = try step2(a);
    Result.Ok(b)
}

func main() -> lang.i64 {
    let result = pipeline(15);
    match result {
        .Ok(_) => 0,
        .Err(e) => e
    }
}
