// test: execution
// backends: cranelift,llvm
// stdlib: true

// `&` binders in a `for` pattern borrow into each item instead of copying
// it out.
module Test

@main
func main() -> lang.i64 {
    let pairs = [(1, 2), (3, 4)];
    var sum = 0;
    for (&a, b) in pairs {
        sum += a + b;
    }
    if sum != 10 { return 1; }
    0
}
