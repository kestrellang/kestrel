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

func addThree(a: Result[lang.i64, lang.i64], b: Result[lang.i64, lang.i64], c: Result[lang.i64, lang.i64]) -> Result[lang.i64, lang.i64] {
    let x = try a;
    let y = try b;
    let z = try c;
    Result.Ok(lang.i64_add(lang.i64_add(x, y), z))
}

func main() -> lang.i64 {
    let result = addThree(Result.Err(88), Result.Ok(20), Result.Ok(12));
    match result {
        .Ok(_) => 0,
        .Err(e) => e
    }
}
