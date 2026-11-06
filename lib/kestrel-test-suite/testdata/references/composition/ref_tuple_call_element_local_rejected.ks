// test: diagnostics
// stdlib: true

// A ref-returning call kept in a `&T` tuple slot is still escape-checked:
// a slice of a LOCAL array cannot leave the frame.
module Test

import std.collections.(Slice)

func leak(xs: Array[Int64]) -> (&Slice[Int64], &Slice[Int64]) {
    let local = [1, 2, 3];
    (local(..<1), xs(0..)) // ERROR(E494)
}
