// test: diagnostics
// stdlib: false

module Test

protocol Math {
    func add(other: Self) -> Self
    func subtract(other: Self) -> Self
    func multiply(other: Self) -> Self
}
func compute[T](a: T, b: T, c: T) -> T where T: Math {
    var sum: T = a.add(b);
    var diff: T = sum.subtract(c);
    return diff.multiply(a)
}
func main() {}
