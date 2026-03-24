// test: diagnostics
// stdlib: false

module Test
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
