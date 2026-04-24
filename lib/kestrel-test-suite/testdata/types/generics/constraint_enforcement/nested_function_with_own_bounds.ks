// test: diagnostics
// stdlib: false

module Test

protocol Printable {
    func print() -> lang.str
}
func outer[T](x: T) -> lang.str where T: Printable {
    return x.print()
}
func main() {}
