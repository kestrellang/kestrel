// test: diagnostics
// stdlib: false

module Test

struct Counter {
    var count: lang.i64

    var doubled: lang.i64 {
        lang.i64_mul(self.count, 2)
    }
}

func test() -> lang.i64 {
    let c = Counter(count: 5);
    lang.i64_add(c.doubled, 10)
}
