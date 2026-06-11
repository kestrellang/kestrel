// test: diagnostics
// stdlib: false

// Named-binding lifetime rules: returning a binding rooted at a local is
// the stage-1 escape error (E494) — INCLUDING after the binding threaded
// through a control-flow merge (references "1.75": bindings cross blocks
// as @guaranteed block args; the threaded param carries the original
// provenance root, so the escape checker still sees the Local root).
// Legal cross-block uses are pinned by bindings/cross_block_*.ks.
module Test

func dangleBinding() -> &lang.i64 {
    var x: lang.i64 = 5;
    let r = &x;
    return r // ERROR(E494)
}

func dangleAfterMerge(c: lang.i1) -> &lang.i64 {
    var x: lang.i64 = 1;
    let r = &x;
    if c {
        x = 2;
    }
    return r // ERROR(E494)
}
