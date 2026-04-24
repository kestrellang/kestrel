// test: diagnostics
// stdlib: false

module Main

func test() -> lang.str {
    let x = 42;
    x // ERROR: does not conform to protocol
}
