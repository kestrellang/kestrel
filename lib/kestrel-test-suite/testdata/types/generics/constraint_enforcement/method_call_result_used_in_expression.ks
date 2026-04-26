// test: diagnostics
// stdlib: false

module Test

protocol Valuable {
    func value() -> lang.i64
}
func sumValues[T](a: T, b: T) -> lang.i64 where T: Valuable {
    var x: lang.i64 = a.value();
    var y: lang.i64 = b.value();
    return x
}
func main() {}
