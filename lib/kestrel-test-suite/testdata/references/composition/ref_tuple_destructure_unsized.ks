// test: execution
// stdlib: true

// Destructuring a tuple of `&Slice[T]` binds each element as a ref. The
// slots hold addresses, so the pattern copies them; it must not plan a
// partial move of the NotCopyable pointee (that read the fat pointer as a
// moved-out `Slice` and crashed).
module Test

import std.collections.(Slice)

@main
func main() {
    let arr = [1, 2, 3, 4, 5];
    let lo = arr(..<2);
    let hi = arr(2..);
    let t: (&Slice[Int64], &Slice[Int64]) = (lo, hi);
    let (a, b) = t;
    print("\(a.count) \(b.count) \(a(0) + b(2))");
}

// CHECK: 2 3 6
