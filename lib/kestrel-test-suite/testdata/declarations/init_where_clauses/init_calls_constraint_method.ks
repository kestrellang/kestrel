// test: diagnostics
// stdlib: false

module Test

protocol Hashable {
    func hash() -> lang.i64
}

struct HashedContainer {
    var hashValue: lang.i64

    init[T](value: T) where T: Hashable {
        self.hashValue = value.hash()
    }
}
