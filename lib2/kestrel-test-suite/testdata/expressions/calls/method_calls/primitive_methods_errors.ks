// test: diagnostics
// stdlib: false

module Main

func test(x: lang.i64) -> () {
    let f = x.toString; // ERROR: primitive method 'toString' on 'I64' must be called
    x.notAMethod // ERROR: cannot access member on type
}
