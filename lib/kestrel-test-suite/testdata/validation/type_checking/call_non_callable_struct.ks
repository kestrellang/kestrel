// test: diagnostics
// stdlib: false

module Main
struct S { }
func test() {
    let s = S();
    s(); // ERROR
}
