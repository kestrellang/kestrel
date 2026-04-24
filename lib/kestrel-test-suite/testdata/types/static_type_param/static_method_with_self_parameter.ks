// test: diagnostics
// stdlib: false

module Test

protocol Factory {
    static func combine(a a: Self, b b: Self) -> Self
}
func merged[T](x: T, y: T) -> T where T: Factory {
    return T.combine(a: x, b: y)
}
