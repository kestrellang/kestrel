// test: diagnostics
// stdlib: false

// E603 through a PROJECTION, the sibling of
// assign_to_capture_in_normal_kind_still_e603.ks (which writes the whole
// captured local).
//
// docs/design/closures.md, "Normal: read-only views": a normal closure's
// captures are read-only, and the copy rule depends on it — "copies share the
// frame's views, which is sound because nobody writes through them". The view
// tier binds each capture's ADDRESS, so `c.n = 5` would write straight back
// through the view and break exactly that argument. The check therefore has to
// catch any assignment target ROOTED at a capture, not only a bare local
// (`assign_target_root` in kestrel-analyze body/closure.rs).
//
// `mutating` is the answer the fix-it note points at, and it lifts the check —
// see mutating_body_assigns_captures_no_e603.ks.
module Test

struct C {
    var n: lang.i64
}

func test() -> lang.i64 {
    var c = C(n: 1);
    let f: () -> lang.i64 = {
        c.n = 5;    // ERROR(E603)
        c.n
    };
    f()
}
