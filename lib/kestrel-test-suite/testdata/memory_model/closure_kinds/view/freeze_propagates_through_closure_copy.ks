// test: diagnostics
// stdlib: false

// docs/design/closures.md, "The freeze rule": "capture provenance propagates
// through closure copies ... every binding or temporary that may carry the view
// extends the freeze to the end of its own lexical extent; copying `f` to a
// wider-scoped `g` therefore extends the freeze through `g`" — E507.
module Test

struct Res: not Copyable {
    var v: lang.i64
}

func sink(consuming r: Res) {}

func test() -> lang.i64 {
    let r = Res(v: 1);
    let f = { r.v };
    let g = f;      // the copy carries the view — freeze now runs through `g`
    sink(r);        // ERROR(E507)
    g()
}
