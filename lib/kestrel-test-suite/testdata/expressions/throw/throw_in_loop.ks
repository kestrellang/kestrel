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
func findOrFail(items: lang.i64) -> Result[lang.i64, Error] {
    var i: lang.i64 = 0;
    while lang.i64_signed_lt(i, items) {
        if lang.i64_eq(i, 5) {
            throw Error()
        }
        i = lang.i64_add(i, 1);
    }
    Result.Ok(i)
}
