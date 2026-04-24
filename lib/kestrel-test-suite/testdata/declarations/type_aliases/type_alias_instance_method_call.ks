// test: diagnostics
// stdlib: false

module Test

struct Counter {
    var count: lang.i64

    init(c: lang.i64) {
        self.count = c;
    }

    func getCount() -> lang.i64 {
        self.count
    }
}

type C = Counter

func test() -> lang.i64 {
    let c: C = Counter(42);
    c.getCount()
}
