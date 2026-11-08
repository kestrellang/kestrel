// test: execution
// stdlib: true
// expect-exit: 0

// The synthesized closure captures free variables like a written one.

module Test

func add(a: Int64, b: Int64) -> Int64 { a + b }

func makeShift(by n: Int64) -> escaping (Int64) -> Int64 {
    add(_, n)
}

@main
func main() -> lang.i64 {
    let threshold = 2;
    let xs = [1, 2, 3, 4];
    let big = xs.filter(where: _ > threshold);
    if big.count != 2 { return 1 }
    let s = makeShift(by: 10);
    if s(1) != 11 { return 2 }
    0
}
