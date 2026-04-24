// test: diagnostics
// stdlib: false

module Test

struct Box[T] { }
protocol Factory[T] {
    func create() -> Box[T]
}
