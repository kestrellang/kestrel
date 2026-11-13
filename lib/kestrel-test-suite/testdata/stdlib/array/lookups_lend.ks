// test: execution
// stdlib: true
// expect-stdout: 3 5 -1 4 1 1 5 9 true 3 | Some(3) None Some(4) true false | cd Some(ab) 7 9\n

// Lookups lend `Optional[&T]` instead of copying. A lent optional works in
// every value position: a binding copies it out, and so do operators (`??`,
// `==`, including against a literal `.Some(3)`), interpolation, call
// arguments and method receivers (`.map`). A match borrows the element.
module Test

import std.collections.(Array, Deque)

struct P { var x: Int64 }

func show[V](v: V) -> String where V: Formattable { "\(v)" }

@main
func main() -> lang.i64 {
    let xs = [3, 1, 4, 1, 5];
    let f = xs.first;
    let l = xs.last;
    let e: [Int64] = [];
    let w = xs.find(where: { it > 3 });
    let lw = xs.findLast(where: { it < 3 });
    let mn = xs.min();
    let mx = xs.max();
    let ps = [P(x: 2), P(x: 9)];
    let px = match ps.last { .Some(p) => p.x, .None => 0 };
    let same = xs.first == .Some(3);
    print("\(f ?? 0) \(l ?? 0) \(e.first ?? -1) \(w ?? 0) \(lw ?? 0) \(mn ?? 0) \(mx ?? 0) \(px) \(same) \(xs.first!)");
    print(" | \(xs.first) \(xs(checked: 9)) \(xs.first.map({ it + 1 })) \(xs.first.isSome) \(xs.first == xs.last)");
    let names = ["ab", "cd"];
    var d = Deque[Int64](from: [7, 8]);
    d.append(9);
    print(" | \(names.last ?? "") \(show(names.first)) \(d.first ?? 0) \(d.last ?? 0)\n");
    0
}
