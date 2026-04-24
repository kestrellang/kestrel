// test: diagnostics
// stdlib: false

module Main

func test() {
    let t = (1, 2);
    let x = t.5; // ERROR: out of bounds
}
