// test: diagnostics
// stdlib: false

// docs/design/closures.md, "The freeze rule (view kinds only)": destruction is
// frozen regardless of HOW it is spelled. A `consuming self` method call takes
// ownership of the receiver just like a consuming argument does, so it must be
// rejected with E507 while a view-kind closure still captures the receiver.
module Test

struct Res: not Copyable {
    var v: lang.i64

    consuming func take() -> lang.i64 { self.v }
}

func test() -> lang.i64 {
    let r = Res(v: 1);
    let f = { r.v };        // view of `r.v`
    let n = r.take();       // ERROR(E507)
    lang.i64_add(n, f())
}
