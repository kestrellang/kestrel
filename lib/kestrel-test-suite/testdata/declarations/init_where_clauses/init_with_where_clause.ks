// test: diagnostics
// stdlib: false

module Test

protocol Countable {
    func count() -> lang.i64
}

struct Wrapper {
    var total: lang.i64

    init[T](item: T) where T: Countable {
        self.total = item.count()
    }
}
