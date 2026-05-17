// test: diagnostics
// stdlib: false
// include: try_prelude.ks

module Test
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
        .Err(e) => e.data
    }
}
