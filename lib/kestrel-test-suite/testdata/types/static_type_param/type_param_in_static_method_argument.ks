// test: diagnostics
// stdlib: false

module Test

protocol Combinable {
    static func combine(left left: Self, right right: Self) -> Self
}
func merged[T](a: T, b: T) -> T where T: Combinable {
    return T.combine(left: a, right: b)
}
