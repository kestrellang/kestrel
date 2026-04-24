// test: diagnostics
// stdlib: false

module Test
@dummy
enum Result {
    case Success(value: lang.i64)
    case Failure(message: lang.str)
}
