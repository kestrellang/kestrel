// test: diagnostics
// stdlib: true

module Main

protocol Hashable {
    func hash() -> lang.i64
}

struct MySet[T]: Cloneable where T: Hashable, T: Cloneable {
    let items: [T]

    func clone() -> MySet[T] {
        MySet(items: self.items.clone())
    }
}
