// test: diagnostics
// stdlib: false

module Test

enum Result[T, E = lang.str] {
    case Ok(value: T)
    case Error(error: E)
}
