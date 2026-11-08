// test: execution
// stdlib: true
// expect-stdout: 99 10 7 7 3 -1 3 49\n

// `d(i)` and `d(checked: i)` are places: compound assignment and member
// writes go through `mutating ref` in place, a shared copy forks first (the
// fork re-bases the ring, so the slot is located after it), and
// `d(checked: i)` lends `Optional[&T]`.
module Test
import std.collections.(Deque)
struct P { var x: Int64 }
func total(d: Deque[P]) -> Int64 { d(0).x + d(1).x }
@main
func main() -> lang.i64 {
    var d = Deque[Int64](from: [1, 2, 3]);
    d.pushFront(0);
    d(1) = 10;
    d(2) += 5;
    let a = d(checked: 3);
    let b = d(checked: 4);
    if let .Some(&mutating v) = d(checked: 0) { v = 99; }
    let copy = d;
    d(3) = 7;
    var ps = Deque[P](from: [P(x: 1), P(x: 2)]);
    ps(0).x += 40;
    if let .Some(&mutating p) = ps(checked: 1) { p.x = 8; }
    print("\(d(0)) \(d(1)) \(d(2)) \(d(3)) \(a ?? -1) \(b ?? -1) \(copy(3)) \(total(ps))\n");
    0
}
