// test: diagnostics
// stdlib: false
// include: ../try_operator/try_prelude.ks

module Test
struct ErrorA {}
struct ErrorB {}
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
func test(cond: lang.i1) -> Result[lang.i64, ErrorA] {
    if cond {
        throw ErrorA()
    } else {
        throw ErrorA()
    }
}
