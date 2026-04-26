// test: diagnostics
// stdlib: false

module Test

struct Box[T] {
    func identity[T](value: T) -> T { value } // ERROR: shadows
}
