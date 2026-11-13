// test: diagnostics
// stdlib: false
// include: ../try_operator/try_prelude.ks

module Test
struct Error {
    var message: lang.str
}
enum Result[T, E] {
    @builtin(.ResultOkCase)
    case Ok(T)
    @builtin(.ResultErrCase)
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
