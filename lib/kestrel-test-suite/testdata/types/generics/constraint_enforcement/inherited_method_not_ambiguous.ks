// test: diagnostics
// stdlib: false

module Test

protocol Base {
    func baseMethod() -> lang.i64
}
protocol Child1: Base {
    func child1Method() -> lang.i64
}
protocol Child2: Base {
    func child2Method() -> lang.i64
}
func useBase[T](x: T) -> lang.i64 where T: Child1, T: Child2 {
    return x.baseMethod()
}
func main() {}
