// test: diagnostics
// stdlib: false

module Test

protocol A {
    func methodA() -> lang.i64
}
protocol B {
    func methodB() -> lang.i64
}
func wrong[T](x: T) -> lang.i64 where T: A, T: B {
    return x.methodC() // ERROR: methodC
}
func main() {}
