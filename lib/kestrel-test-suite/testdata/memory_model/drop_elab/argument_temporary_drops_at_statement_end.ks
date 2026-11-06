// test: execution
// backends: cranelift,llvm
// expect-exit: 0

// A temporary passed by borrow to a call (`peek(mk(1))`) is destroyed when
// the statement ends, not at the end of the enclosing block. It used to live
// to the block's end.

module Test

var drops: Int64 = 0

struct R: not Copyable {
    var v: Int64
    deinit { drops = drops + 1; }
}

func mk(v: Int64) -> R { R(v: v) }
func peek(r: R) -> Int64 { r.v }

func run() -> Int64 {
    peek(mk(1));
    let afterFirst = drops;
    peek(mk(2));
    if afterFirst != 1 { return 1 }
    if drops != 2 { return 2 }
    0
}

@main
func main() -> lang.i64 { if run() != 0 { return 1 } 0 }
