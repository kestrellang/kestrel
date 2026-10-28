// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0

// Regression (#193): a ref-returning protocol requirement (`peek() -> &Int64`)
// dispatched through a generic bound must return the pointee VALUE, not the
// address. The witness thunk previously lost the ret_borrow deref on the
// caller side: binding decay (`let v = x.peek()`) treated the returned pointer
// as the value on both backends, and the operator path (`x.peek() + 0`) was
// ASLR garbage on cranelift. Both paths must now read 21. `// backends:` is
// load-bearing — the two backends diverged on the broken behavior.

module Test

protocol Peekable {
    func peek() -> &Int64
}

struct Box: Peekable {
    var v: Int64
    func peek() -> &Int64 { self.v }
}

func decayThrough[T](x: T) -> Int64 where T: Peekable {
    let v = x.peek();   // binding decay through witness dispatch
    v
}

func opThrough[T](x: T) -> Int64 where T: Peekable {
    x.peek() + 0        // operator path through witness dispatch
}

@main
func main() -> lang.i32 {
    if decayThrough(Box(v: 21)) != 21 { return 1 }
    if opThrough(Box(v: 21)) != 21 { return 2 }
    return 0;
}
