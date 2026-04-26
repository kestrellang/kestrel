// test: diagnostics
// stdlib: false

module Test

protocol A {
    func same() -> lang.i64
}
protocol B {
    func same() -> lang.i64
}
func ambig[T](x: T) -> lang.i64 where T: A and B {
    return x.same() // ERROR: ambiguous
}
func main() {}
