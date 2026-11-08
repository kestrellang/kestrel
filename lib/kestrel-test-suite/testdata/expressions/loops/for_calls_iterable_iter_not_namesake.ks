// test: execution
// stdlib: true
// expect-exit: 6
//
// `for x in bag` calls the `Iterable.iter` requirement — not whatever member
// is named `iter`. `Bag` also witnesses `Walk.iter()` (same name and labels,
// declared first), which must never be picked by a `for` loop: 1+2+3 = 6.

module Test

protocol Walk {
    func iter() -> Int64
}

struct CountUp: Iterator {
    type Item = Int64
    var i: Int64
    var n: Int64
    mutating func next() -> Int64? {
        if self.i >= self.n { return .None; }
        self.i = self.i + 1;
        .Some(self.i)
    }
}

struct Bag: Iterable, Walk {
    type Item = Int64
    type TargetIterator = CountUp
    var n: Int64
    func iter() -> Int64 { 100 }
    func iter() -> CountUp { CountUp(i: 0, n: self.n) }
}

@main
func main() -> Int64 {
    var total: Int64 = 0;
    for x in Bag(n: 3) {
        total = total + x;
    }
    total
}
