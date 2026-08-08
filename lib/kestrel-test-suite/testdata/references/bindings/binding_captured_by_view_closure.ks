// test: execution
// stdlib: false
// expect-exit: 0

// E212 IS RETIRED (docs/design/closures.md, Diagnostics table: "E212 —
// closures cannot capture reference bindings — retired: view capture is now
// the default"; plan lockstep 6). This file used to assert that rejection.
//
// The old rule existed because an env that STORED the reference could outlive
// the borrow. A view-kind (normal / `mutating`) environment cannot: it holds
// addresses into the frame it was created in and is frame-bound for its whole
// life (E494 rejects every escape route). A ref binding is itself already a
// place, so the view captures its TARGET — which is what this test pins:
// reading through the captured binding sees the CURRENT value of `x`, not a
// snapshot taken at capture time.
//
// The owning tier still rejects it, with its own code — see
// memory_model/closure_kinds/escaping/owning_capture_of_ref_binding_rejected.ks
// (E624).
module Test

@main
func main() -> lang.i64 {
    var x: lang.i64 = 1;
    let r = &x;
    let cl = { () in lang.i64_add(r, 1) };
    if lang.i64_eq(cl(), 2) { } else { return 1; }
    x = 41;
    // the capture is a VIEW of the referent, not a copy taken at capture time
    if lang.i64_eq(cl(), 42) { } else { return 2; }
    0
}
