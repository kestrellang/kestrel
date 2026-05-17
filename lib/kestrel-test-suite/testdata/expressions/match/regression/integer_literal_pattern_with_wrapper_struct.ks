// test: diagnostics
// stdlib: false

module Main

@builtin(.ExpressibleByIntLiteral)
protocol ExpressibleByIntLiteral {
    init(intLiteral value: lang.i64)
}

struct MyInt: ExpressibleByIntLiteral {
    var value: lang.i64

    init(intLiteral value: lang.i64) {
        self.value = value
    }
}

func test_match(x: MyInt) -> lang.i64 {
    match x {
        0 => 100,
        1 => 200,
        _ => 300,
    }
}
