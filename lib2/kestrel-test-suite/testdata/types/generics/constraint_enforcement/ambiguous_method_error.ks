// test: diagnostics
// stdlib: false

module Test

protocol P1 {
    func doIt() -> lang.i64
}
protocol P2 {
    func doIt() -> lang.i64
}
func ambig[T](x: T) -> lang.i64 where T: P1, T: P2 {
    return x.doIt() // ERROR: ambiguous
}
func main() {}
