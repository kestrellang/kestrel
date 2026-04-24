// test: diagnostics
// stdlib: false

module Test

protocol Unused {
    func unused() -> lang.i64
}
func ignoreConstraint[T](x: lang.i64) -> lang.i64 where T: Unused {
    return x
}
func main() {}
