// test: diagnostics
// stdlib: true

module Test

func test() {
    []; // ERROR: could not infer type
}
