// test: diagnostics
// stdlib: false

module Main
struct S {
    var x: lang.i64
}
func test() {
    let s = S(x: 1);
    s.x = 2 // ERROR: cannot assign to immutable field 'x'
}
