// test: execution
// stdlib: false
// expect-exit: 0

// `Self(...)` works in methods declared directly inside a struct body
// (no `extend` block) — the lookup walks parents and finds the enclosing
// `Struct` directly.

module Main

struct Counter {
    public let n: lang.i64
    public init(n n: lang.i64) { self.n = n }

    public static func zero() -> Self {
        Self(n: 0)
    }
}

@main
func main() -> lang.i64 {
    let z = Counter.zero();
    z.n
}
