// test: diagnostics
// stdlib: false
module Test
enum Container[T] {
    case Single(value: T)
    case Multiple(values: T)
}
