// test: diagnostics
// stdlib: true

module Test

protocol Summable {
    func sumWith(others: [Self]) -> Self
}
func sumAll[T](x: T, others: [T]) -> T where T: Summable {
    return x.sumWith(others)
}
func main() {}
