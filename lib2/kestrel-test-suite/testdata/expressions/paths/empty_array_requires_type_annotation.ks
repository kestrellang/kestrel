// test: diagnostics
// stdlib: false

module Test

func test() {
    []; // ERROR: could not infer type
}
