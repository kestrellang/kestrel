// test: diagnostics
// stdlib: true

module Main

protocol Hashable {
    func hash() -> lang.i64
}

struct MySet[T] where T: Hashable {
    let items: [T]
}
