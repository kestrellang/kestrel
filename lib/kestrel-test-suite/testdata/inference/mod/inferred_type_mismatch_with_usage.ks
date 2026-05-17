// test: diagnostics
// stdlib: false

module Main

func test() {
    let x = 42;
    let y: lang.str = x; // ERROR: does not conform to protocol
}
