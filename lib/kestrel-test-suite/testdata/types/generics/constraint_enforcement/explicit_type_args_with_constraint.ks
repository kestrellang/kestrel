// test: diagnostics
// stdlib: false

module Test

protocol Addable {
    func add(other: Self) -> Self
}
func double[T](x: T) -> T where T: Addable {
    return x.add(x)
}
func caller[U](y: U) -> U where U: Addable {
    return double[U](y)
}
func main() {}
