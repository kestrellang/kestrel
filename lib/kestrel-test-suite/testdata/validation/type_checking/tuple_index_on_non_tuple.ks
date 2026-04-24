// test: diagnostics
// stdlib: false

module Main

func test() {
    let x = 42;
    let y = x.0; // ERROR: cannot index into non-tuple type
}
