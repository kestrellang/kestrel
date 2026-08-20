// test: diagnostics
// stdlib: false
//
// G8: `while true { break; }` in a guard-else does NOT diverge — the break
// exits the loop and control falls straight through the guard with the
// condition false. The E003 gate used to be `HirExpr::Loop { .. } => true`
// with no break check, so this compiled and `test(0)` returned 99.

module Main

func test(x: lang.i64) -> lang.i64 {
    guard lang.i64_signed_gt(x, 0) else {
        while true { break; } // ERROR: guard else block must diverge
    }
    99
}
