// test: execution
// stdlib: true
// expect-exit: 0

// A type can adopt `ExpressibleByArrayLiteral` on its own: the protocol's
// default bridge turns the compiler's pointer/count call into
// `init(arrayLiteral:)`. The bridge was commented out, so only Array worked.
module Test

struct Bag: ExpressibleByArrayLiteral {
    type Element = Int64
    var sum: Int64
    init(arrayLiteral elements: LiteralSlice[Int64]) {
        var s = 0;
        for e in elements { s += e; }
        self.sum = s;
    }
}

@main
func main() -> Int64 {
    let b: Bag = [1, 2, 3];
    b.sum - 6
}
