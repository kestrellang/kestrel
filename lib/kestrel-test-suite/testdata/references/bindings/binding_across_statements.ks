// test: execution
// stdlib: false
// backends: cranelift,llvm

// The headline named-binding capability: a returned ref HELD across
// statement boundaries — multiple reads through one borrow, no false
// E497 at statement ends. The binding stays block-local: its last use
// precedes any control flow.
module Test

struct Box {
    var v: lang.i64
    func peek() -> &lang.i64 { self.v }
}

@main
func main() -> lang.i64 {
    let b = Box(v: 7);
    let r = &b.peek();
    let first = r;
    let second = lang.i64_add(r, 1);
    let third = r;
    if lang.i64_eq(first, 7) { } else { return 1; }
    if lang.i64_eq(second, 8) { } else { return 2; }
    if lang.i64_eq(third, 7) { } else { return 3; }
    0
}
