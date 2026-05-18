// test: diagnostics
// stdlib: false

module Test

indirect enum Container[T] { // ERROR: indirect enums are not yet supported
    case Single(value: T)
    case Nested(inner: Container[T])
}
