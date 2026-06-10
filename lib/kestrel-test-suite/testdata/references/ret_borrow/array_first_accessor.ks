// test: execution
// stdlib: true
// backends: cranelift,llvm

// PointerDerived propagation end-to-end: Array.at fabricates the ref from
// its storage pointer internally (`.value`, root PointerDerived), while
// the caller-side result is scoped to the array — the safe discipline
// outside, the unchecked step inside the stdlib.
module Test

@main
func main() -> lang.i64 {
    let arr = [11, 22, 33];
    if arr.at(index: 0) != 11 { return 1; }
    if arr.at(index: 1) != 22 { return 2; }
    if arr.at(index: 2) != 33 { return 3; }
    0
}
