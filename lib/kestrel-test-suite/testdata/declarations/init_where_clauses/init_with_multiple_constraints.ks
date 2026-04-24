// test: diagnostics
// stdlib: false

module Test

protocol Hashable {
    func hash() -> lang.i64
}

protocol Comparable {
    func compare(other: Self) -> lang.i64
}

struct Storage {
    var value: lang.i64

    init[T](item: T) where T: Hashable, T: Comparable {
        self.value = item.hash()
    }
}
