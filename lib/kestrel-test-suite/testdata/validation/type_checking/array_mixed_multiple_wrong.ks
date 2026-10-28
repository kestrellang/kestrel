// test: diagnostics
// stdlib: true

module Main

func test() {
    let arr: [lang.i64] = [
        1,
        "two", // ERROR: does not satisfy constraint: i64 !: ExpressibleByStringLiteral
        true,  // ERROR: does not satisfy constraint: i64 !: ExpressibleByBoolLiteral
        4.0,   // ERROR: does not satisfy constraint: i64 !: ExpressibleByFloatLiteral
    ];
}
