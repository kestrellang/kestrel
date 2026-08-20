// test: diagnostics
// stdlib: false
//
// G8: the hole was not `while`-specific. A bare `loop { break; }` is the same
// `HirExpr::Loop` node and must be rejected for the same reason — the break
// exits it, so the guard-else falls through.

module Main

func test(x: lang.i64) -> lang.i64 {
    guard lang.i64_signed_gt(x, 0) else {
        loop { break; } // ERROR: guard else block must diverge
    }
    99
}
