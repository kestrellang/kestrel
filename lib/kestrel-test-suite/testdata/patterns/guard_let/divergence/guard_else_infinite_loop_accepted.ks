// test: diagnostics
// stdlib: false
//
// The other side of G8: a genuinely infinite `loop {}` has no break, never
// falls through, and must stay accepted. Tightening E003 to check for a break
// must not turn this into a false reject.

module Main

func test(x: lang.i64) -> lang.i64 {
    guard lang.i64_signed_gt(x, 0) else {
        loop {}
    }
    99
}
