// test: diagnostics
// stdlib: false

module Test
@dummy
protocol Container[T] {
    func read() -> T
}
