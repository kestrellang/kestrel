// test: diagnostics
// stdlib: false

module Test

protocol Calculator {
    func calculate(left left: lang.i64, right right: lang.i64) -> lang.i64
}
func doCalc[T](calc: T) -> lang.i64 where T: Calculator {
    return calc.calculate(left: 1, right: 2)
}
func main() {}
