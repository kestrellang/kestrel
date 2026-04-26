// test: diagnostics
// stdlib: false

module Main

func takeString(s: lang.str) {}

func test() {
    let x = 42;
    takeString(x) // ERROR: does not conform to protocol
}
