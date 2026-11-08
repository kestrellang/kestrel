// test: execution
// stdlib: true
// expect-exit: 0

// A chain that mutates its payload calls `chainMutate` on a `var` base, so
// the write lands in the optional itself: mutating methods, assignment and
// compound assignment.

module Test

struct Counter {
    var n: Int64
    mutating func bump() { self.n = self.n + 1; }
}

@main
func main() -> lang.i64 {
    var c: Counter? = .Some(Counter(n: 1));

    c?.bump();
    if c?.n != .Some(2) { return 1 }

    c?.n = 10;
    if c?.n != .Some(10) { return 2 }

    c?.n += 5;
    if c?.n != .Some(15) { return 3 }
    0
}
