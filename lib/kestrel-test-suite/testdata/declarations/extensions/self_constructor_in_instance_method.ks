// test: execution
// stdlib: false
// expect-exit: 0

// `Self(...)` works in instance methods, not just static ones — the
// enclosing-type lookup is the same.

module Main

struct Counter {
    public let n: lang.i64
    public init(n n: lang.i64) { self.n = n }
}

extend Counter {
    public func bumped() -> Self {
        Self(n: lang.i64_add(self.n, 1))
    }
}

func main() -> lang.i64 {
    let c = Counter(n: 4);
    let d = c.bumped();
    lang.i64_sub(d.n, 5)
}
