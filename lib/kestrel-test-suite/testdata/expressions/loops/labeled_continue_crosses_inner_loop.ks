// test: execution
// stdlib: true
// expect-exit: 0
//
// #201 (BUG-67): a labeled `continue` that crosses an inner loop to re-enter
// an OUTER loop used to ICE in OSSA verify ("value consumed more than once").
// `lower_continue` fished the header's own param values instead of following
// the positional tracker contract `lower_break` uses, which misaligns once an
// inner loop replaces the active tracker. This checks it both compiles and
// has the right semantics: the labeled continue must skip the rest of the
// inner loop and re-enter the outer loop.

module Test

@main
func main() -> lang.i32 {
    var c: Int64 = 0;
    var hits = 0;
    mid: while c < 3 {
        c = c + 1;
        var d = 0;
        while d < 5 {
            d = d + 1;
            if d == 2 {
                continue mid;
            }
            hits = hits + 1;
        }
    }
    // Each outer iteration: inner runs d=1 (hits++), then d=2 -> continue mid.
    // So hits == 3 and c == 3.
    if c != 3 { return 1 }
    if hits != 3 { return 2 }
    0
}
