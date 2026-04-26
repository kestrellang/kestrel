// test: diagnostics
// stdlib: false

module Test
enum Result {
    @dummy
    case Success(value: lang.i64)
    case Failure(message: lang.str)
}
