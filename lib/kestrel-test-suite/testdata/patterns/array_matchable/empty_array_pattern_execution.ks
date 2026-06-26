// test: execution
// stdlib: true
// expect-exit: 0
//
// #188 (BUG-52) facet a1 — the dangerous one. `[]` must match ONLY a
// zero-length array. The existing `empty_array_pattern.ks` is diagnostics-only
// (stops at VALIDATE), so it never observed that at runtime `[]` silently
// matches a NON-empty array — the worst failure mode (wrong arm, no crash).
// This pins the length test: `[]` lowers to `matchLength() == 0`.

module Test

func isEmpty(arr: [Int64]) -> Int64 {
    match arr {
        [] => 1,
        _ => 0
    }
}

@main
func main() -> lang.i64 {
    let e: [Int64] = [];
    let n: [Int64] = [2, 3];
    if isEmpty(e) != 1 { return 1 }  // empty array must hit the `[]` arm
    if isEmpty(n) != 0 { return 2 }  // non-empty must NOT (a1 miscompile if it does)
    0
}
