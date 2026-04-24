// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    let x: lang.i64 = "wrong1"; // ERROR
    let y: lang.str = 42; // ERROR
    return true // ERROR
}
