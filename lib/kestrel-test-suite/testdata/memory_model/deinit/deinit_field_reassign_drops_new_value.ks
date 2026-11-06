// test: execution
// backends: cranelift,llvm
// expect-stdout: drop 1\ndrop 2\n

// A `deinit` that reassigns a field releases the old value at the assignment
// and the new one when the fields drop afterwards. The drop shim used to
// borrow an SSA self that Cranelift spilled to a temporary and never read
// back, so the stale old field dropped twice and the new one leaked.

module Test

struct R: not Copyable {
    var id: Int64;
    deinit { println("drop \(self.id)"); }
}

struct Holder: not Copyable {
    var a: R;
    deinit { self.a = R(id: 2); }
}

@main
func main() -> lang.i64 {
    let h = Holder(a: R(id: 1));
    deinit h;
    0
}
