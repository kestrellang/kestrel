// test: diagnostics
// stdlib: false

module Test
struct Error {}
enum Result[T, E] {
    case Ok(T)
    case Err(E)
}
extend Result[T, E]: Prelude.FromResidual[E] {
    static func fromResidual(residual: E) -> Result[T, E] {
        Result.Err(residual)
    }
}
func test(cond: lang.i1) -> Result[lang.i64, Error] {
    if cond {
        throw Error()
    }
    Result.Ok(42)
}
