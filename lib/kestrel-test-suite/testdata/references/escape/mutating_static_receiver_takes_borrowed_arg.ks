// test: execution
// stdlib: true
// expect-exit: 0
//
// Passing a borrowing value to a `mutating` method stores it nowhere when
// the receiver's type is `Static` (it can hold no reference). This used to
// be a false E509 when the receiver was a `mutating` PARAMETER, whose
// lifetime outlives the reborrowed local.

module Test

import std.collections.(Array, MutableArraySlice)

struct Filler { var byte: Int64 }

extend Filler {
    mutating func fill(consuming into buf: MutableArraySlice[Int64]) -> Int64 {
        buf.fill(with: self.byte);
        self.byte += 1;
        buf.count
    }
}

func fillTail(mutating f: Filler, consuming into buf: MutableArraySlice[Int64]) -> Int64 {
    var rest = buf;
    f.fill(into: rest.mutatingSlice(1..))
}

@main
func main() -> lang.i64 {
    var xs = [0, 0, 0];
    var f = Filler(byte: 5);
    if fillTail(f, into: xs.asMutableSlice()) != 2 { return 1 }
    if xs(0) != 0 or xs(1) != 5 or xs(2) != 5 { return 2 }
    if f.byte != 6 { return 3 }
    0
}
