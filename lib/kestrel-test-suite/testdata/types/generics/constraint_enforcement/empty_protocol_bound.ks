// test: diagnostics
// stdlib: false

module Test

protocol Empty {}
protocol HasMethod {
    func doIt() -> lang.i64
}
func useEmpty[T](x: T) -> lang.i64 where T: Empty, T: HasMethod {
    return x.doIt()
}
func main() {}
