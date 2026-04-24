// test: diagnostics
// stdlib: false

module Test

protocol Describable {
    func describe() -> lang.str
}
func getDescription[T](x: T) -> lang.str where T: Describable {
    return x.describe()
}
func main() {}
