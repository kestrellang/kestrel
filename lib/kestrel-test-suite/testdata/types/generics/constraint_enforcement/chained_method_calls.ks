// test: diagnostics
// stdlib: false

module Test

protocol Chainable {
    func chain() -> Self
}
func chainMany[T](x: T) -> T where T: Chainable {
    return x.chain().chain().chain()
}
func main() {}
