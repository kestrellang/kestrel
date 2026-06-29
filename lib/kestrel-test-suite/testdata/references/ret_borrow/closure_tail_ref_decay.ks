// test: execution
// stdlib: true
// backends: cranelift,llvm

// Regression (#195): a closure whose tail is a bare ref-returning call must
// decay to the pointee — refs do not cross the closure boundary (a closure can
// never ret_borrow). Before the fix the closure body returned the raw
// @guaranteed ref: type inference rejected it ("expected Int64 got &Int64") or,
// once the type was pinned, MIR returned the @guaranteed value without the
// ret_borrow convention → OSSA verify ICE.
module Test

struct Box {
    var v: Int64
    func peek() -> &Int64 { self.v }
}

@main
func main() -> lang.i64 {
    let b = Box(v: 20);
    let k: () -> Int64 = { b.peek() };
    if k() != 20 { return 1; }
    0
}
