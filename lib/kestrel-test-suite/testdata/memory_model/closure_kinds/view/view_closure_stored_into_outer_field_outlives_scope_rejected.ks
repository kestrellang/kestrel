// test: diagnostics
// stdlib: false

// The scope-depth rule (plan D8) through a FIELD target. The sibling
// view_closure_stored_outlives_captured_scope_rejected.ks pins `g = { r.v }`
// for a bare outer local; storing the same view-carrying closure into a field
// of an outer local (`h.f = { r.v }`) is not checked, because the Assign arm in
// move_tracking.rs only runs `check_outlives` when the target is a bare local
// (`rhs_local`), although `place_root` already handles field targets. `r` dies
// at the end of the inner block and `(h.f)()` reads freed storage. Found in the
// 2026-10 architecture review at 9767d2dc: valgrind invalid read/write.
// EXPECTED TO FAIL until the outlives check runs on the target's place root.
module Test

struct Res: not Copyable {
    var v: lang.i64
}

struct Holder {
    var f: () -> lang.i64
}

func test() -> lang.i64 {
    var h = Holder(f: { () in 0 });   // capture-free literal: carries no view
    if lang.i64_eq(1, 1) {
        let r = Res(v: 9);
        h.f = { r.v };                // ERROR(E507)
    }                                 // `r` dies here; `h.f` would dangle
    (h.f)()
}
