// test: execution
// stdlib: true
// backends: cranelift,llvm
// skip: stage1 — needs Array.mutableAt(index:) (S4)

// A returned `&mutating` passed onward to a `mutating`-convention param:
// the callee's write reaches the original element.
module Test

func bump(mutating x: Int64) {
    x = x + 1;
}

@main
func main() -> lang.i64 {
    var arr = [7, 8];
    bump(arr.mutableAt(index: 0));
    if arr(0) != 8 { return 1; }
    if arr(1) != 8 { return 2; }
    0
}
