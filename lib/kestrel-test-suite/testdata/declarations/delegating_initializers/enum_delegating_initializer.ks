// test: diagnostics
// stdlib: false

module Test

enum Result[T, E] {
    case Ok(T)
    case Err(E)

    init(ok value: T) {
        self = Result.Ok(value)
    }

    init(err error: E) {
        self = Result.Err(error)
    }
}
