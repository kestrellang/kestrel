// test: diagnostics
// stdlib: false

module Test

protocol Add {
    func add(other: Self) -> Self
}
protocol Negate {
    func negate() -> Self
}
func compute[T](a: T, b: T) -> T where T: Add, T: Negate {
    var sum: T = a.add(b);
    return sum.negate()
}
func main() {}
