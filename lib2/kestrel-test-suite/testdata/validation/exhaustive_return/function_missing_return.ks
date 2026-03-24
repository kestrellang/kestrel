// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    let x: lang.i64 = 1;
} // ERROR
