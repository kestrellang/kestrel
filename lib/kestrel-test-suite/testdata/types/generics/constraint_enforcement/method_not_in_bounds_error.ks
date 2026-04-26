// test: diagnostics
// stdlib: false

module Test

protocol Add {
    func add(other: Self) -> Self
}
func bad[T](a: T, b: T) -> T where T: Add {
    return a.subtract(b) // ERROR: no member 'subtract'
}
