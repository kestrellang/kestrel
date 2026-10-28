// test: execution
// backends: cranelift,llvm
// expect-exit: 0

// `deinit x;` works inside nested blocks, not only at the top of a body.
// It used to be a parse error anywhere but the top level.

module Test

var drops: Int64 = 0

struct H: not Copyable {
    var v: Int64
    deinit { drops = drops + self.v; }
}

func f(c: Bool) -> Int64 {
    let h = H(v: 1);
    if c {
        deinit h;
        return drops
    }
    drops + 100
}

func g(c: Bool) {
    var h = H(v: 10);
    if c { deinit h; } else { deinit h; }
}

@main
func main() -> lang.i64 {
    if f(true) != 1 { return 1 }
    // h drops at the end of f(false) after the return value is computed
    if f(false) != 101 { return 2 }
    if drops != 2 { return 3 }
    g(true);
    if drops != 12 { return 4 }
    0
}
