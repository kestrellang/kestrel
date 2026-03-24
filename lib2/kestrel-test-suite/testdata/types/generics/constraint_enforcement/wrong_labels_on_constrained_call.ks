// test: diagnostics
// stdlib: false

module Test

protocol Calculator {
    func calculate(left left: lang.i64, right right: lang.i64) -> lang.i64
}
func doCalc[T](calc: T) -> lang.i64 where T: Calculator {
    return calc.calculate(a: 1, b: 2) // ERROR: calculate
}
func main() {}
