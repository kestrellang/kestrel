// test: execution
// stdlib: true
// backends: cranelift,llvm
// skip: stage1 — needs Array.at(index:) (S4)

// Binding decay is a COPY, not a view: mutating the array after the
// binding must not change the bound value.
module Test

@main
func main() -> lang.i64 {
    var arr = [10, 20, 30];
    let x = arr.at(index: 0);
    arr(0) = 99;
    if x != 10 { return 1; }
    if arr(0) != 99 { return 2; }
    0
}
