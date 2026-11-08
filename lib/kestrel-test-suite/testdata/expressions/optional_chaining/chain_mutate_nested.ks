// test: execution
// stdlib: true
// expect-exit: 0

// Mutation propagates outward: `a?.inner?.bump()` mutates the inner payload,
// so the outer bind must lend `a` mutably too.

module Test

struct Counter {
    var n: Int64
    mutating func bump() { self.n = self.n + 1; }
}

struct Outer {
    var inner: Counter?
}

@main
func main() -> lang.i64 {
    var a: Outer? = .Some(Outer(inner: .Some(Counter(n: 1))));
    a?.inner?.bump();
    if a?.inner?.n != .Some(2) { return 1 }
    a?.inner?.n = 7;
    if a?.inner?.n != .Some(7) { return 2 }
    0
}
