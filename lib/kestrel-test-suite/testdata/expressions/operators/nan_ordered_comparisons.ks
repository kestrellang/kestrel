// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0

// Regression (#157): every ordered comparison against NaN must be false (IEEE
// 754), and `nan == nan` is false. Before the fix Float's `<=`/`>=` were derived
// from the three-valued `compare()`, which collapses NaN to `.Equal`; the
// derivations `compare() != .Greater` / `!= .Less` then wrongly returned true for
// NaN. `<`/`>` (using `== .Less` / `== .Greater`) were correct only by luck. The
// fix implements Less/LessOrEqual/Greater/GreaterOrEqual directly via the IEEE
// intrinsics (false for any NaN operand). Covers Float64 and Float32; both
// backends had the bug.

module Test

@main
func main() -> lang.i32 {
    let nan = Float64.nan;
    let one: Float64 = 1.0;
    if nan < nan  { return 1 };
    if nan <= nan { return 2 };
    if nan > nan  { return 3 };
    if nan >= nan { return 4 };
    if nan <= one { return 5 };
    if nan >= one { return 6 };
    if one <= nan { return 7 };
    if one >= nan { return 8 };
    if nan == nan { return 9 };

    // Float32 path (same template-generated impl)
    let nan32 = Float32.nan;
    let one32: Float32 = 1.0;
    if nan32 <= one32 { return 10 };
    if nan32 >= one32 { return 11 };
    if one32 <= nan32 { return 12 };

    // Sanity: ordinary (non-NaN) comparisons must still work.
    if not (1.0 < 2.0)   { return 20 };
    if not (2.0 <= 2.0)  { return 21 };
    if not (2.0 >= 2.0)  { return 22 };
    if not (3.0 > 2.0)   { return 23 };
    if not (Float64.infinity > 1.0) { return 24 };

    return 0;
}
