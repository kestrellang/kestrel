// test: diagnostics
// stdlib: false

module Test

struct Counter {
    var count: lang.i64

    init(c: lang.i64) {
        self.count = c;
    }

    static func zero() -> Counter {
        Counter(0)
    }
}

type C = Counter

func test() -> lang.i64 {
    let c = C.zero();
    c.count
}
