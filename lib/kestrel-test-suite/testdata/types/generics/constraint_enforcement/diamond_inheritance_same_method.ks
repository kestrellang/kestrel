// test: diagnostics
// stdlib: false

module Test

protocol A {
    func doA() -> lang.i64
}
protocol B: A {}
protocol C: A {}
func diamond[T](x: T) -> lang.i64 where T: B, T: C {
    return x.doA()
}
func main() {}
