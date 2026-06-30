// test: execution
// stdlib: true
// expect-exit: 0
//
// #201 (BUG-67): soundness companion. A non-Copyable value owned inside the
// inner loop body must be dropped exactly once when a labeled `continue`
// crosses out of the inner loop — not double-dropped (the OSSA "consumed more
// than once" failure mode) and not leaked. The labeled continue threads the
// outer loop's tracked values back to its header while destroying the crossed
// inner scopes.

module Test

var DROPS: Int64 = 0;

struct Res {
    var id: Int64;
    deinit { DROPS = DROPS + 1; }
}

@main
func main() -> lang.i32 {
    var c = 0;
    mid: while c < 3 {
        c = c + 1;
        var d = 0;
        while d < 5 {
            d = d + 1;
            let r = Res(id: d); // owned, non-Copyable, fresh each inner iter
            if d == 2 {
                continue mid;   // crosses inner loop; r must drop exactly once
            }
            // normal path: r drops at end of inner-body scope
        }
    }
    // Per outer iter the inner loop runs d=1 (r drops at scope end) then d=2
    // (continue, r drops): 2 drops/iter * 3 iters = 6.
    if DROPS != 6 { return 1 }
    0
}
