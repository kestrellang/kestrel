// test: diagnostics
// stdlib: false

module Main
func test() {
    var x: lang.i64;
    let y = x; // ERROR: access to uninitialized variable 'x'
}
