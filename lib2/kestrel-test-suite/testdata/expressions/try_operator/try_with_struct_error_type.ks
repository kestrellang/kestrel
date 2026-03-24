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
        .Err(e) => e.code
    }
}
