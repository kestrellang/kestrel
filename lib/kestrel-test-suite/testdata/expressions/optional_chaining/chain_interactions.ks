// test: execution
// stdlib: true
// expect-exit: 0

// `?.` with its neighbours: `|>` ends the chain and pipes its Optional, a
// chain in a closure mutates a captured `var`, and an annotated binding
// promotes a plain field read (`let x: Int64? = s.count`).

module Test

struct Counter {
    var n: Int64
    mutating func bump() { self.n = self.n + 1; }
}

func orZero(x: Int64?) -> Int64 { x ?? 0 }

@main
func main() -> lang.i64 {
    let c: Counter? = .Some(Counter(n: 4));
    if (c?.n |> orZero) != 4 { return 1 }

    var d: Counter? = .Some(Counter(n: 1));
    var bumpIt = { d?.bump(); };
    bumpIt();
    if d?.n != .Some(2) { return 2 }

    let plain = Counter(n: 9);
    let x: Int64? = plain.n;
    if x != .Some(9) { return 3 }
    0
}
