// test: diagnostics
// stdlib: false

// docs/design/closures.md, "The freeze rule (view kinds only)": a frozen place
// "cannot be moved". Rebinding a non-Copyable value with `let moved = r;` moves
// it out from under the live view, so it is an E507 freeze violation — NOT the
// old E500 (the view never moved `r` in the first place).
module Test

struct Res: not Copyable {
    var v: lang.i64
}

func test() -> lang.i64 {
    let r = Res(v: 1);
    let f = { r.v };      // view of `r.v`
    let moved = r;        // ERROR(E507)
    lang.i64_add(moved.v, f())
}
