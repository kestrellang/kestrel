// test: execution
// backends: cranelift,llvm
// stdlib: true

// A `for` pattern can destructure a reference item (`refs()` /
// `mutableRefs()`) the way a `match` on a ref binding does: the pattern
// matches the pointee, and a `&mutating` binder writes the element in
// place. Nested patterns look through a ref-typed
// payload (the loop's `.Some(item)`).
module Test

@main
func main() -> lang.i64 {
    let pairs = [(1, 2), (3, 4)];
    var sum = 0;
    for (a, b) in pairs.refs() {
        sum += a + b;
    }
    if sum != 10 { return 1; }

    var cells = [(1, 0), (2, 0)];
    for (&mutating x, _) in cells.mutableRefs() {
        x += 10;
    }
    if cells(0).0 != 11 { return 2; }
    if cells(1).0 != 12 { return 3; }
    0
}
