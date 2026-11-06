// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-stdout: 5 12 None 5\n50 1 4 9 7\n13 3 3 4 13\n8 10 7 \nchunks 3 c 2 10 next true\n

// Array's one generic subscript: `xs(i)` is the element place, `xs(r)` a
// `&Slice[T]` over the same storage. Both read and write in place, and the
// views (`chunks`) are built over a slice.
module Test
import std.collections.(Array, Slice)

func sum(s: Slice[Int64]) -> Int64 {
    var t: Int64 = 0;
    for x in s { t += x; }
    t
}

@main
func main() -> Int64 {
    var xs: Array[Int64] = [5, 3, 9, 1, 7];
    print("\(xs(0)) \(sum(xs(1..<3))) \(xs(checked: 9)) \(xs.count)\n");
    xs(0) = 50;
    xs(1) += 1;
    xs(1..<4).sort();
    print("\(xs(0)) \(xs(1)) \(xs(2)) \(xs(3)) \(xs(4))\n");

    var ys: Array[Int64] = [5, 3, 9, 1, 7];
    let mid = &ys(1..<4);
    let implicit = ys(1..<4);
    print("\(sum(mid)) \(mid.count) \(mid(0)) \(ys(3) + implicit(0)) \(implicit.count + 10)\n");
    for c in ys.chunks(of: 2) { print("\(sum(c)) "); }
    print("\n");

    let s = &ys(0..<5);
    let v = s.chunks(of: 2);
    let c = &v(1);
    var it = v.iter();
    let n = it.next();
    print("chunks \(v.count) c \(c.count) \(sum(c)) next \(n.isSome)\n");
    0
}
