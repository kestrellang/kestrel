// test: diagnostics
// stdlib: false

module Test

protocol Container[T] {
    func read() -> T
}
func extract[C, T](c: C) -> T where C: Container[T] {
    return c.read()
}
func main() {}
