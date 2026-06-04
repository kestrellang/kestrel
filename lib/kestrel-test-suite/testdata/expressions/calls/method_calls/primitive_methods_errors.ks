// test: diagnostics
// stdlib: false

module Main

func test(x: lang.i64) -> () {
    let f = x.toString; // ERROR: method 'toString' on 'i64' must be called
    x.notAMethod // ERROR: cannot access member on type
}
