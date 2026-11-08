// test: execution
// stdlib: true
// expect-exit: 0

// Everything after a `?.` runs only when the bind succeeds: call arguments
// and later bases are skipped on `.None`.

module Test

struct Counter {
    var count: Int64
}

struct Adder {
    var base: Int64
    func add(n: Int64) -> Int64 { self.base + n }
}

func bump(mutating c: Counter) -> Int64 {
    c.count = c.count + 1;
    c.count
}

@main
func main() -> lang.i64 {
    var calls = Counter(count: 0);
    let none: Adder? = .None;
    let r = none?.add(bump(calls));
    if r != .None { return 1 }
    if calls.count != 0 { return 2 }

    let full: Adder? = .Some(Adder(base: 10));
    let s = full?.add(bump(calls));
    if s != .Some(11) { return 3 }
    if calls.count != 1 { return 4 }
    0
}
