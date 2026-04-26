// test: diagnostics
// stdlib: false

module Test
struct Error {
    var message: lang.str
}
enum Result[T, E] {
    case Ok(T)
    case Err(E)
}
extend Result[T, E]: Prelude.FromResidual[E] {
    static func fromResidual(residual: E) -> Result[T, E] {
        Result.Err(residual)
    }
}
func failing() -> Result[lang.i64, Error] {
    throw Error(message: "something went wrong")
}
