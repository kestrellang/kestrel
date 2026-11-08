// test: execution
// stdlib: true
// expect-exit: 6
//
// A `for` loop advances its iterator through the `Iterator.next` requirement.
// `CountUp` also witnesses `Step.next()` (same name and labels, declared
// first, returns Bool), which the loop must never call: 1+2+3 = 6.

module Test

protocol Step {
    mutating func next() -> Bool
}

struct CountUp: Iterator, Step {
    type Item = Int64
    var i: Int64
    var n: Int64
    mutating func next() -> Bool { false }
    mutating func next() -> Int64? {
        if self.i >= self.n { return .None; }
        self.i = self.i + 1;
        .Some(self.i)
    }
}

@main
func main() -> Int64 {
    var total: Int64 = 0;
    for x in CountUp(i: 0, n: 3) {
        total = total + x;
    }
    total
}
