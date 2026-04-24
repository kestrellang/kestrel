// test: diagnostics
// stdlib: false

module Test

protocol Hashable {
    func hash() -> lang.i64
}

struct Point { }

extend Point: Hashable {
    func hash() -> lang.i64 { 42 }
}
