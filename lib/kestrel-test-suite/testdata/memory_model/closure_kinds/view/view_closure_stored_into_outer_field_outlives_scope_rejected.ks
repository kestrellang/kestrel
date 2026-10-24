// test: diagnostics
// stdlib: false

// The scope-depth rule (plan D8) through a FIELD target. The sibling
// view_closure_stored_outlives_captured_scope_rejected.ks pins `g = { r.v }`
// for a bare outer local; storing the same view-carrying closure into a field
// of an outer local (`h.f = { r.v }`) is checked against the target's place
// root (`h`), so it is E507 too. Since closure fields must be owning, the
// view-closure field itself is also E626 (docs/design/closure-fields.md).
module Test

struct Res: not Copyable {
    var v: lang.i64
}

struct Holder {
    var f: () -> lang.i64 // ERROR(E626)
}

func test() -> lang.i64 {
    var h = Holder(f: { () in 0 });   // capture-free literal: carries no view
    if lang.i64_eq(1, 1) {
        let r = Res(v: 9);
        h.f = { r.v };                // ERROR(E507)
    }                                 // `r` dies here; `h.f` would dangle
    (h.f)()
}
