// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-stdout: true false true\n

// The right side of `or` is a closure that captures `s`, a borrowed
// unsized `Slice[Int64]`. The capture is the slice's (fat) address. It used
// to copy the slice's "bits" out of an uninitialized one-byte slot, so
// `s.count` read garbage on the right side.
module Test
import std.collections.(Array, Slice)

func outside(i: Int64, s: Slice[Int64]) -> Bool {
    i < 0 or i >= s.count
}

@main
func main() -> Int64 {
    var xs: Array[Int64] = [1, 2, 3, 4, 5];
    let s = &Slice.lend(start: xs.asUnsafeSlice().pointer, count: 5);
    print("\(outside(9, s)) \(outside(2, s)) \(outside(5, s))\n");
    0
}
