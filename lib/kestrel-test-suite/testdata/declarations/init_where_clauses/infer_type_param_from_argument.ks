// test: diagnostics
// stdlib: false

module Test

protocol Countable {
    func count() -> lang.i64
}

struct List: Countable {
    var size: lang.i64
    func count() -> lang.i64 { self.size }
}

struct Counter {
    var total: lang.i64

    init[T](items: T) where T: Countable {
        self.total = items.count()
    }
}

func test() -> Counter {
    let list = List(size: 5);
    Counter(list)
}
