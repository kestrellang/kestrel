// test: diagnostics
// stdlib: false

module Main

enum Result {
    case Ok(value: lang.i64)
    case Err(message: lang.str)
}
