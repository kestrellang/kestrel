// test: diagnostics
// stdlib: true

module Main

func test() {
    let arr: [lang.i64] = [
        1,
        "two", // ERROR: expected i64 got String
        true,  // ERROR: expected i64 got Bool
        4.0,   // ERROR: expected i64 got Float64
    ];
}
