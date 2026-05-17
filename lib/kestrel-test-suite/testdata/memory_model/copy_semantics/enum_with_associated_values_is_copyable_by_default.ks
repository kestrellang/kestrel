// test: diagnostics
// stdlib: false

module Test

enum Result {
    case Ok(value: lang.i64)
    case Err(code: lang.i64)
}
