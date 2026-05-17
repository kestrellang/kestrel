// test: diagnostics
// stdlib: false

module Test

protocol Getter {
    func read() -> lang.i64
}

struct Box[T]: Getter {
    func read() -> lang.i64 { 42 }
}
