// test: diagnostics
// stdlib: false
// include: try_prelude.ks

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

func compute(r: Result[lang.i64, lang.i64]) -> Result[lang.i64, lang.i64] {
    let value = try r;
    Result.Ok(lang.i64_mul(value, 2))
}

func main() -> lang.i64 {
    let result = compute(Result.Ok(21));
    match result {
        .Ok(v) => v,
        .Err(_) => 0
    }
}
