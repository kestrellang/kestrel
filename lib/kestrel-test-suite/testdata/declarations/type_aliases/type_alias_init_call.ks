// test: diagnostics
// stdlib: false

module Test

struct Counter {
    var count: lang.i64

    init(c: lang.i64) {
        self.count = c;
    }
}

type C = Counter

func test() -> lang.i64 {
    let c = C(42);
    c.count
}
