// test: diagnostics
// stdlib: true

module Main
func test() {
    let x: lang.i64 = null; // ERROR: does not conform
}
