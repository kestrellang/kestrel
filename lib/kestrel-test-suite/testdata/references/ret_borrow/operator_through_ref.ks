// test: execution
// stdlib: true
// backends: cranelift,llvm

// Operator dispatch peels the ref: `arr(0) == 42` resolves
// Equatable on the pointee and borrows the place for the receiver.
module Test

@main
func main() -> lang.i64 {
    let arr = [42, 7];
    if not (arr(0) == 42) { return 1; }
    if arr(1) == 42 { return 2; }
    0
}
