// test: execution
// stdlib: false
// expect-exit: 0

// docs/design/closures.md, "The freeze rule (view kinds only)": "Plain
// reassignment stays legal — that is a write through a live view, which
// references already permit." Pins that whole-place assignment of a viewed
// aggregate compiles AND that the view then reads the new value.
module Test

struct P {
    var a: lang.i64
    var b: lang.i64
}

@main
func main() -> lang.i64 {
    var p = P(a: 1, b: 2);
    let f = { p.a };            // view of the place `p.a`
    p = P(a: 10, b: 20);        // legal: a write, not a destruction
    if lang.i64_eq(f(), 10) { } else { return 1; }
    p.a = 30;                   // field write through the same live view
    if lang.i64_eq(f(), 30) { } else { return 2; }
    0
}
