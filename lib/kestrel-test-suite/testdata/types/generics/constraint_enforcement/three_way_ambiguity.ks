// test: diagnostics
// stdlib: false

module Test

protocol A {
    func common() -> lang.i64
}
protocol B {
    func common() -> lang.i64
}
protocol C {
    func common() -> lang.i64
}
func threeWay[T](x: T) -> lang.i64 where T: A, T: B, T: C {
    return x.common() // ERROR: ambiguous
}
func main() {}
