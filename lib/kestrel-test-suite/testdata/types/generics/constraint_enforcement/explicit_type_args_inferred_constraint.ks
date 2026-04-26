// test: diagnostics
// stdlib: false

module Test

protocol Process {
    func run() -> Self
}
func inner[T](x: T) -> T where T: Process {
    return x.run()
}
func outer[U](y: U) -> U where U: Process {
    var result: U = inner[U](y);
    return result
}
func main() {}
