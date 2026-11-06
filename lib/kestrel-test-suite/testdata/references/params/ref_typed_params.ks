// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-stdout: 2 4 3 3 5 13 6\n

// `x: &T` and `x: &mutating T` are parameters passed by borrow / mutable
// borrow, like `x: T` and `mutating x: T`. The same spelling works in
// function types and closure params, and a `&S` param may take an unsized
// type such as a slice.
module Test

import std.collections.(Array, Sliceable)

struct Big { var xs: Array[Int64] }

func apply(b: &Big, f: (&Big) -> Int64) -> Int64 { f(b) }
func bump(b: &mutating Big, f: (&mutating Big) -> ()) { f(b) }
func first(b: &Big) -> &Int64 { b.xs(0) }
func setFirst(b: &mutating Big, to v: Int64) { b.xs(0) = v }

func total[S](s: &S) -> Int64 where S: Sliceable[Int64], S: not Copyable, S: not Sized {
    var t: Int64 = 0;
    for i in 0..<s.count { t += s(i) }
    t
}

@main
func main() -> Int64 {
    var b = Big(xs: [3, 4]);
    let n = apply(b, { (x: &Big) in x.xs.count });
    let m = apply(b, { (x) in x.xs(1) });
    bump(b, { (x: &mutating Big) in x.xs.append(9) });
    print("\(n) \(m) \(b.xs.count) \(first(b)) ");
    print("\(apply(b, { (x: &Big) in 5 })) ");
    setFirst(b, to: 2);
    print("\(total(b.xs(1..<3))) \(total(b.xs(0..<2)))\n");
    0
}
