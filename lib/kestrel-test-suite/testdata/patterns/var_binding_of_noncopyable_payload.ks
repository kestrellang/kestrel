// test: execution
// backends: cranelift,llvm
// expect-stdout: drop 5\n

// A `var` pattern binding of a non-copyable payload is a mutable local:
// writes through it update the binding (it used to be an immutable SSA value
// that the first write consumed, an OSSA error). It drops once.

module Test

struct R: not Copyable {
    var v: Int64;
    mutating func bump() { self.v = self.v + 1; }
    deinit { println("drop \(self.v)"); }
}

func f(consuming o: Optional[R]) -> Int64 {
    match o {
        .Some(var r) => {
            r.v = r.v + 1;
            r.bump();
            r.v
        },
        .None => 0
    }
}

@main
func main() -> lang.i64 {
    if f(.Some(R(v: 3))) != 5 { return 1 }
    0
}
