// test: diagnostics
// stdlib: false

module Main

enum Result[T, E] {
    case Ok(value: T)
    case Err(error: E)
}
