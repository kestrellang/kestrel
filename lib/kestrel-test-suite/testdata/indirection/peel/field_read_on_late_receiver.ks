// test: execution
// stdlib: true
// expect-stdout: 3 3\n

// A field read passed to a borrowed param peels through `Indirection` even
// when its receiver is typed late: `n` is an unannotated literal, so
// `n.digits` resolves only after literal defaulting. The argument waits for
// its own type instead of being pinned to the param's.
module Test

import std.collections.(Array, Slice)
import std.core.(Indirection)

struct Digits { let xs: Array[Int64] }

extend Digits: Indirection {
    type Target = Slice[Int64]
    var value: Slice[Int64] { ref { self.xs.asSlice() } }
}

extend Int64 {
    var digits: Digits { Digits(xs: [1, 2, 3]) }
}

func count(xs: &Slice[Int64]) -> Int64 { xs.count }

@main
func main() -> lang.i64 {
    let n = 7;
    let m: Int64 = 7;
    print("\(count(n.digits)) \(count(m.digits))\n");
    0
}
