// test: diagnostics
// stdlib: false

module Main

func test() {
    let f = { it }; // ERROR: could not infer type
}
