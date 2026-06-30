// test: execution
// stdlib: true
// backends: cranelift,llvm

// Regression (#194): an explicit `return e;` statement is a value position,
// identical to the return-tail — a ref-returning call decays to the pointee
// when the declared return type is non-ref. Before the fix only the bare tail
// decayed; `return b.peek();` reported "expected Int64 got &Int64".
module Test

struct Box {
    var v: Int64
    func peek() -> &Int64 { self.v }
}

func stmtReturn(b: Box) -> Int64 { return b.peek(); }   // return statement decays
func tailReturn(b: Box) -> Int64 { b.peek() }           // tail still decays

@main
func main() -> lang.i64 {
    let b = Box(v: 9);
    if stmtReturn(b) != 9 { return 1; }
    if tailReturn(b) != 9 { return 2; }
    0
}
