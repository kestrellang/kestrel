// test: execution
// stdlib: true

// A tuple literal whose `&T` slots are pinned by the return type keeps the
// refs of ref-returning CALL elements (not just local reads): `xs(..<at)`
// lends a `&Slice[Int64]` into the tuple instead of decaying (E109). Both
// the tail and a `return` statement are checked against the return type.
module Test

import std.collections.(Slice)

func split(xs: Array[Int64], at: Int64) -> (&Slice[Int64], &Slice[Int64]) {
    if at <= 0 {
        return (xs(..<0), xs(0..))
    }
    (xs(..<at), xs(at..))
}

func firstTwo(xs: Array[Int64]) -> (&Int64, Int64) {
    (xs(0), xs(1))
}

@main
func main() {
    let arr = [1, 2, 3, 4, 5];
    let (a, b) = split(arr, 2);
    if a.count != 2 or b.count != 3 or a(1) != 2 or b(0) != 3 {
        fatalError("split tail broke: \(a.count) \(b.count)");
    }
    let (c, d) = split(arr, 0);
    if c.count != 0 or d.count != 5 {
        fatalError("split return broke: \(c.count) \(d.count)");
    }
    let t = split(arr, 4);
    if t.0.count + t.1.count != 5 {
        fatalError("tuple access broke");
    }
    let (r, n) = firstTwo(arr);
    if r + n != 3 {
        fatalError("sized ref slot broke: \(r + n)");
    }
    print("ok");
}

// CHECK: ok
