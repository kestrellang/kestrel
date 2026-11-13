// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-stdout: direct 5 2 9\nvia fn 5 7 1\n

// A `&Slice[T]` stored in a struct field is a fat pointer (data, count):
// reading the field yields the slice itself, not a thin pointer to it.
module Test
import std.collections.(Array, Slice)

struct Holder { let s: &Slice[Int64]; let k: Int64 }

func mk(s: Slice[Int64]) -> Holder {
    let r = &s;
    Holder(s: r, k: 7)
}

@main
func main() -> Int64 {
    var xs: Array[Int64] = [5, 3, 9, 1, 7];
    let s = &xs(0..<5);
    let h = Holder(s: s, k: 2);
    print("direct \(h.s.count) \(h.k) \(h.s(2))\n");
    let h2 = mk(s);
    print("via fn \(h2.s.count) \(h2.k) \(h2.s(3))\n");
    0
}
