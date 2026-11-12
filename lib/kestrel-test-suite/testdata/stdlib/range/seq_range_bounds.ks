// test: execution
// stdlib: true
// expect-exit: 0
// expect-stdout: 2..<5 2..<4 2..<5 0..<3 0..<4\n[1, 4, 5, 6]\n
//
// `SeqRange` conformers state their ends as `RangeBound`s; `resolve(count:)`
// turns them into a half-open range against a length, and `removeSubrange`
// takes any range form through it.

module Test

import std.collections.(SeqRange, RangeBound)

func show[R](range: R) -> String where R: SeqRange {
    let r = range.resolve(5);
    "\(r.start)..<\(r.end)"
}

@main
func main() {
    print("\(show(2..<5)) \(show(2..=3)) \(show(2..)) \(show(..<3)) \(show(..=3))\n");
    var xs = [1, 2, 3, 4, 5, 6];
    xs.removeSubrange(1..=2);
    print("\(xs)\n");
}
