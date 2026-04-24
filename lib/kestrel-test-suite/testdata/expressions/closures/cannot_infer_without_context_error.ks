// test: diagnostics
// stdlib: false

module Main

func test() {
    let f = { (x) in x }; // ERROR: could not infer type
}
