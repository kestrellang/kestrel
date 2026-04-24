// test: diagnostics
// stdlib: false

module Test

struct Square {
    var side: lang.i64

    var area: lang.i64 {
        lang.i64_mul(self.side, self.side)
    }
}

func test() -> lang.i64 {
    let s = Square(side: 5);
    s.area
}
