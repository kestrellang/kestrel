// test: execution
// backends: cranelift,llvm
// expect-exit: 0

// Elements after `..` in a tuple pattern bind from the END of the tuple, so
// `(a, .., b)` on a 3-tuple binds `b` to the last element, not the second.

module Test

func last(t: (Int64, Int64, Int64)) -> Int64 {
    match t { (_, .., b) => b }
}

@main
func main() -> lang.i64 {
    if last((5, 2, 6)) != 6 { return 1 }
    let t = (1, 2, 3, 4);
    match t {
        (.., c, d) => { if c != 3 or d != 4 { return 2 } },
    }
    let (a, .., z) = t;
    if a != 1 or z != 4 { return 3 }
    let nested = (10, (7, 8, 9));
    match nested {
        (x, (.., y)) => { if x != 10 or y != 9 { return 4 } },
    }
    0
}
