// test: execution
// stdlib: true
// expect-exit: 0

// Everything except the hole is inside the closure body, so it is
// evaluated on every call, not when the closure is made.

module Test

struct Counter {
    var n: Int64
}

func tick(mutating c: Counter) -> Int64 {
    c.n = c.n + 1;
    c.n
}

func add(a: Int64, b: Int64) -> Int64 { a + b }

@main
func main() -> lang.i64 {
    var c = Counter(n: 0);
    var f: mutating (Int64) -> Int64 = add(_, tick(c));
    if c.n != 0 { return 1 }
    if f(10) != 11 { return 2 }
    if f(10) != 12 { return 3 }
    if c.n != 2 { return 4 }
    0
}
