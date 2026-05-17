// test: diagnostics
// stdlib: false

module Main

struct Container[T] {
    let item: T
    func isEmpty() -> lang.i1 { false }
}

struct Wrapper[T] {
    let value: T
    func getValue() -> T { self.value }
    func isEqual(other: T) -> lang.i1 { false }
}
