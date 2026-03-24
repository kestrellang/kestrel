// test: diagnostics
// stdlib: false

module Test

protocol Processable {
    func process() -> Self
}
func helper[T](x: T) -> T where T: Processable {
    return x.process()
}
func outer[U](y: U) -> U where U: Processable {
    var result: U = helper[U](y);
    return result
}
func main() {}
