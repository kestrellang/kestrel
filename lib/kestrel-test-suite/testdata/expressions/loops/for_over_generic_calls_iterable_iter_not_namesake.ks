// test: execution
// stdlib: true
// expect-exit: 6
//
// Inside a generic `T: Walk, Iterable`, `for x in t` must call the
// `Iterable.iter` requirement through the bound — the `Walk.iter()` bound
// requirement has the same name and labels and comes first: 1+2+3 = 6.

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

func sum[T](items: T) -> Int64 where T: Walk, T: Iterable, T.Item = Int64 {
    var total: Int64 = 0;
    for x in items {
        total = total + x;
    }
    total
}

@main
func main() -> Int64 {
    sum(Bag(n: 3))
}
