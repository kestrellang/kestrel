// test: execution
// stdlib: true
// backends: cranelift,llvm

// Operator dispatch peels the ref: `arr.at(index: 0) == 42` resolves
// Equatable on the pointee and borrows the place for the receiver.
module Test

@main
func main() -> lang.i64 {
    let arr = [42, 7];
    if not (arr.at(index: 0) == 42) { return 1; }
    if arr.at(index: 1) == 42 { return 2; }
    0
}
