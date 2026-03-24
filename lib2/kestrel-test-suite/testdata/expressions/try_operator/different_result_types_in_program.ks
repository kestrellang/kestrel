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
    let resultA = computeA(Result.Ok(16));
    let a = match resultA {
        .Ok(v) => v,
        .Err(_) => 0
    };

    let resultB = computeB(Result.Ok(8));
    let b = match resultB {
        .Ok(v) => v,
        .Err(_) => 0
    };

    lang.i64_add(a, b)
}
