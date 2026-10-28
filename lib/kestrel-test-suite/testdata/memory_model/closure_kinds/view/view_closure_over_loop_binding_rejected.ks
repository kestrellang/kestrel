// test: diagnostics
// stdlib: true

// docs/design/closures.md, "Pinned Edge Cases": "a normal or `mutating`
// closure capturing a loop iteration binding cannot outlive that iteration's
// lexical scope".
//
// The iteration binding is the SHORTEST-LIVED place in the program: `for i in
// …` desugars to `loop { match $iter.next() { .Some(i) => body, .None =>
// break } }` (hir-lower `desugar_for_loop`), so `i` is a match-arm binding
// whose storage dies at the end of each iteration. Assigning a view of it to
// `g` — declared OUTSIDE the loop — is the scope-depth half of the freeze rule
// (plan D8): no move and no `deinit` happens anywhere, so only the outlives
// comparison catches it. Without it `g()` below reads dead storage with no
// diagnostic at all.
//
// The fix-it is an owning kind; the positive sibling is
// escaping_closure_over_loop_binding_accumulates.ks.
module Test

import std.numeric.Int64

func test() -> Int64 {
    var g: () -> Int64 = { () in 0 };
    for i in 1..=3 {
        g = { i * 10 };   // ERROR(E507)
    }
    g()
}
