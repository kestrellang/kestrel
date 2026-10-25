// test: execution
// stdlib: true
// expect-exit: 0

// On `.None` a mutating chain does nothing, and the right side of an
// assignment through `?.` is not evaluated.

module Test

struct Counter {
    var n: Int64
    mutating func bump() { self.n = self.n + 1; }
}

struct Tally {
    var calls: Int64
}

func next(mutating t: Tally) -> Int64 {
    t.calls = t.calls + 1;
    t.calls
}

@main
func main() -> lang.i64 {
    var c: Counter? = .None;
    var tally = Tally(calls: 0);
    c?.bump();
    c?.n = next(tally);
    c?.n += next(tally);
    if tally.calls != 0 { return 1 }
    if c?.n != .None { return 2 }
    0
}
