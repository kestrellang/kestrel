// test: execution
// backends: cranelift,llvm
// stdlib: true

// Stage 2d stdlib surface: `Array.refs()` yields shared references to
// the elements in place. Works on literal-inferred arrays (the
// pattern-binder gate keeps loop-variable uses from racing the
// element-literal defaulting) and heap-payload elements (String) —
// reads through the ref, no element copies.
module Test

@main
func main() -> lang.i64 {
    let a = [10, 20, 30];
    var sum = 0;
    for x in a.refs() {
        sum = sum + x;
    }
    if sum != 60 { return 1; }

    let words = ["alpha", "beta"];
    var letters = 0;
    for w in words.refs() {
        letters = letters + w.byteCount;
    }
    if letters != 9 { return 2; }
    0
}
