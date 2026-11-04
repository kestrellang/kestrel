// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-stdout: count 5 first 1 sum 15 max Some(5)\nsub 3 2 9 7 3\nchecked None Some(2) clamped 9 Some(5) None wrapped Some(5)\nafter 7 8 103 4 50\n

// `Slice[T]` lends its elements and subslices in place through one generic
// subscript: `s(i)` is a `&T`, `s(r)` a narrower `&Slice[T]` over the same
// elements, and both are writable through `&mutating`.
module Test
import std.collections.(Array, Slice)

func sum(s: Slice[Int64]) -> Int64 {
    var t: Int64 = 0;
    for x in s { t += x; }
    t
}

func bump(mutating s: Slice[Int64]) { s(0) += 100; }

func show(xs: Array[Int64]) -> String {
    var out = "";
    for x in xs { out = out + " \(x)"; }
    out
}

@main
func main() -> Int64 {
    var xs: Array[Int64] = [1, 2, 3, 4, 5];
    let p = xs.asUnsafeSlice().pointer;
    let s = &Slice.lend(start: p, count: 5);
    print("count \(s.count) first \(s(0)) sum \(sum(s)) max \(s.max())\n");
    let t = &s(1..<4);
    print("sub \(t.count) \(t(0)) \(sum(t)) \(sum(t(1..))) \(sum(s(..=1)))\n");
    let c = &s(clamped: 3..<99);
    let e = &s(0..<0);
    print("checked \(s(checked: 9)) \(s(checked: 1)) clamped \(sum(c)) \(s(clamped: 99)) \(e(clamped: 0)) wrapped \(s(wrapped: -1))\n");
    let m = &mutating Slice.lendMutating(start: p, count: 5);
    m(4) = 50;
    bump(m(2..<5));
    m(0..<2).assign(from: [7, 8]);
    print("after\(show(xs))\n");
    0
}
