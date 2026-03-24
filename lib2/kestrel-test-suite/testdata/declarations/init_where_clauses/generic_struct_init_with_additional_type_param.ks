// test: diagnostics
// stdlib: false

module Test

protocol Converter[To] {
    func convert() -> To
}

struct Box[T] {
    var value: T

    init[From](from: From) where From: Converter[T] {
        self.value = from.convert()
    }
}
