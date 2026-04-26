// test: diagnostics
// stdlib: false

module Main
struct S {
    let x: lang.i64
}
func test() {
    var s = S(x: 1);
    s.x = 2 // ERROR: cannot assign to immutable field 'x'
}
