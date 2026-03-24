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
        .Ok(v) => v,
        .Err(_) => 0
    }
}
