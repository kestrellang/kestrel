// test: diagnostics
// stdlib: true

module Main

func test() {
    let d = [:]; // ERROR: could not infer type
}
