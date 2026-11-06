// test: diagnostics
// stdlib: true

module Test

// A `for` loop over a value that is already an `Iterator` uses it as the
// loop's iterator directly (no `iter()` call) — so a non-Copyable iterator is
// MOVED into the loop and can't be used afterwards.

struct Countdown: Iterator, not Copyable {
    type Item = Int64
    var n: Int64
    mutating func next() -> Int64? {
        if self.n == 0 { return .None }
        self.n -= 1;
        .Some(self.n)
    }
}

func run() -> Int64 {
    let c = Countdown(n: 3);
    var total = 0;
    for v in c { total += v; }
    total + c.n // ERROR(E500)
}
