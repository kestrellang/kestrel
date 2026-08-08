// test: diagnostics
// stdlib: false

// docs/design/closures.md, "The freeze rule (view kinds only)": an explicit
// `deinit r;` destroys the place early, which is exactly what a live view
// forbids. Pins the third destruction spelling (alongside moves and consuming
// arguments) as an E507 freeze violation.
module Test

struct Res: not Copyable {
    var v: lang.i64
}

func test() -> lang.i64 {
    let r = Res(v: 10);
    let f = { r.v };   // view of `r.v`
    deinit r;          // ERROR(E507)
    f()
}
