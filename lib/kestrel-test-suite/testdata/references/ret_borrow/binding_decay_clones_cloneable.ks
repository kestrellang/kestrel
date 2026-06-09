// test: execution
// stdlib: true
// backends: cranelift,llvm
// skip: stage1 — needs Array.at(index:) (S4)

// Decay of a Cloneable (heap String) element retains/clones — no aliasing,
// no double-free (the string_literal_return_no_alias class). Both the bound
// copy and the mutated array must survive scope exit cleanly.
module Test

@main
func main() -> lang.i64 {
    var arr = ["alpha", "beta"];
    let s = arr.at(index: 0);
    arr(0) = "gamma";
    if s != "alpha" { return 1; }
    if arr(0) != "gamma" { return 2; }
    0
}
