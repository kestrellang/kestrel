// test: diagnostics
// stdlib: false

module Test

protocol Combine {
    func combine(with other: Self) -> Self
}
func merge[T](a: T, b: T) -> T where T: Combine {
    return a.combine(with: b)
}
