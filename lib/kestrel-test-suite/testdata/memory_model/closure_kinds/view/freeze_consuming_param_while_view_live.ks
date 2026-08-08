// test: diagnostics
// stdlib: false

// docs/design/closures.md, "The freeze rule (view kinds only)": while a live
// normal-kind closure carries a view of `r`, `r` is frozen against destruction
// — passing it to a `consuming` parameter would leave the view dangling. This
// is the design's own `consume(file)` example, and it is the new E507.
module Test

struct Res: not Copyable {
    var v: lang.i64
}

func sink(consuming r: Res) {}

func test() -> lang.i64 {
    let r = Res(v: 3);
    let f = { r.v };   // view of `r.v` — freezes `r` for the rest of the scope
    sink(r);           // ERROR(E507)
    f()
}
