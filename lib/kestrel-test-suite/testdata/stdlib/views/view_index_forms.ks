// test: execution
// stdlib: true
// expect-exit: 0
// expect-stdout: él él lo hé hé\ntrue true false true\n[hé] [lo] [] []\n[lo] [él] [héllo] [héll] [o]\nSome(o) Some(h) Some(o) None\n2 1 2 true\ntwo|2|Some(three)|2|None\néll 4\n
//
// Every index form (Int64 and the five range types) through every access
// mode (plain, checked, within, wrapped, clamped) on the bytes, chars and
// lines views. One set of bounds rules serves all views, so each view must
// agree on the same positions.

module Test

@main
func main() {
    let s: String = "h\u{00E9}llo";
    let c = s.chars;
    let a = c(1..<3); let b = c(1..=2); let d = c(3..); let e = c(..<2); let f = c(..=1);
    print("\(a.toOwned()) \(b.toOwned()) \(d.toOwned()) \(e.toOwned()) \(f.toOwned())\n");
    let k1 = c(checked: 4..<6); let k2 = c(checked: 5..); let k3 = c(checked: ..=4); let k4 = c(checked: 3..<2);
    print("\(k1.isNone) \(k2.isSome) \(k3.isNone) \(k4.isNone)\n");
    let w1 = c(within: (-3)..<2); let w2 = c(within: 3..=99); let w3 = c(within: 9..); let w4 = c(within: ..<(-1));
    print("[\(w1.toOwned())] [\(w2.toOwned())] [\(w3.toOwned())] [\(w4.toOwned())]\n");
    let r1 = c(wrapped: (-2)..); let r2 = c(wrapped: 6..<8); let r3 = c(wrapped: ..=(-1)); let r4 = c(wrapped: ..<(-1)); let r5 = c(wrapped: 4..=9);
    print("[\(r1.toOwned())] [\(r2.toOwned())] [\(r3.toOwned())] [\(r4.toOwned())] [\(r5.toOwned())]\n");
    let i1 = c(wrapped: -1); let i2 = c(clamped: -4); let i3 = c(clamped: 40); let i4 = c(checked: 5);
    print("\(i1) \(i2) \(i3) \(i4)\n");

    let by = s.bytes;
    let y1 = by(1..=2); let y2 = by(wrapped: (-1)..); let y3 = by(within: 4..<10); let y4 = by(checked: 0..=6);
    print("\(y1.count) \(y2.count) \(y3.count) \(y4.isNone)\n");

    let l = "one\ntwo\r\nthree".lines;
    let l1 = l(1); let l2 = l(1..).count; let l3 = l(wrapped: -1); let l4 = l(within: ..<2).count; let l5 = l(checked: 3);
    print("\(l1)|\(l2)|\(l3)|\(l4)|\(l5)\n");
    print("\(c.substring(1..=3)) \(l.substring(..<1).bytes.count)\n");
}
