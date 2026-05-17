// test: diagnostics
// stdlib: false

module Test

indirect enum Container[T] {
    case Single(value: T)
    case Nested(inner: Container[T])
}
