// test: execution
// stdlib: true
// expect-stdout: 3 6 9 0 4 12 1\n

// A borrowed argument peels whatever produced it: an array literal the
// param's type can't take (it defaults to `Array`, then peels to `Slice`),
// a method call's result, a generic call's result, an init argument, a
// defaulted parameter's value, and an overload picked only through the peel.
module Test

import std.collections.(Array, Slice)
import std.memory.(OwnedBox)

func total(xs: &Slice[Int64]) -> Int64 {
    var t: Int64 = 0;
    for x in xs { t += x }
    t
}

func totalOr(xs: &Slice[Int64] = Array[Int64]()) -> Int64 { total(xs) }

func id[T](x: T) -> T { x }

struct Sum {
    var n: Int64
    init(of xs: &Slice[Int64]) { self.n = total(xs); }
}

struct Size {
    var n: Int64
    init(of s: &Slice[Int64]) { self.n = s.count; }
}
extend Size {
    init(of s: &String) { self.n = 100; }
}

func balance(of b: &Int64) -> Int64 { b }

@main
func main() {
    let xs = [4, 5, 3];
    let raw = xs.asUnsafeSlice();
    print("\(total([1, 2])) \(total(xs.asUnsafeSlice()) - 6) \(total(id([4, 5])))");
    print(" \(totalOr()) \(Sum(of: [1, 3]).n) \(Size(of: raw).n * 4)");
    println(" \(balance(of: OwnedBox(1)))");
}
