// test: diagnostics
// stdlib: false

module Test

enum Result[T, E] {
    case Ok(value: T)
    case Error(error: E)
}
