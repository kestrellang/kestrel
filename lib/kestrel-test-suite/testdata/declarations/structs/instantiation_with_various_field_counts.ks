// test: diagnostics
// stdlib: false
module Test
struct Single {
    var value: lang.i64
}

struct Many {
    var a: lang.i64
    var b: lang.i64
    var c: lang.i64
    var d: lang.i64
    var e: lang.i64
}

func makeSingle() -> Single {
    Single(value: 42)
}

func makeMany() -> Many {
    Many(a: 1, b: 2, c: 3, d: 4, e: 5)
}
