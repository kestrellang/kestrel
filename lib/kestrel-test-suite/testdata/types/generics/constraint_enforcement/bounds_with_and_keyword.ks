// test: diagnostics
// stdlib: false

module Test

protocol A {
    func doA() -> lang.i64
}
protocol B {
    func doB() -> lang.i64
}
func both[T](x: T) -> lang.i64 where T: A and B {
    var a: lang.i64 = x.doA();
    var b: lang.i64 = x.doB();
    return a
}
func main() {}
