// test: diagnostics
// stdlib: false

// docs/design/closures.md, "The freeze rule" + "Pinned Edge Cases": capture
// provenance propagates through aggregate construction, so a struct holding a
// view-kind closure field keeps the captured place frozen for the struct's
// lexical extent. The closure literal itself is only a temporary here — the
// freeze must survive into `h`, giving E507 on the consuming call.
module Test

struct Res: not Copyable {
    var v: lang.i64
}

struct Holder {
    let f: () -> lang.i64
}

func sink(consuming r: Res) {}

func test() -> lang.i64 {
    let r = Res(v: 2);
    let h = Holder(f: { r.v });   // frame-view field: `Holder` is frame-bound
    sink(r);                      // ERROR(E507)
    (h.f)()
}
