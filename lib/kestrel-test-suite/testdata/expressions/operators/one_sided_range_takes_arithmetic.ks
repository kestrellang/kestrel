// test: execution
// stdlib: true
// expect-exit: 0
// expect-stdout: 4 2 2 4 2\ntrue\n
//
// A one-sided range groups like a binary range missing an end: postfix
// `x + 1..` is `(x + 1)..` and prefix `..<x - 1` is `..<(x - 1)`, so the
// arithmetic stays inside the range; `-2..` starts at -2.

module Test

@main
func main() {
    let x: Int64 = 1;
    let xs = [10, 20, 30, 40, 50, 60];
    let a = xs(x + 1..);
    let b = xs(..<x + 1);
    let c = xs(..=x * 1);
    let w = xs.asSlice()(within: x - 3..);
    let s: String = "hello";
    let tail = s.chars(wrapped: -2..);
    print("\(a.count) \(b.count) \(c.count) \(tail.count + 2) \(w.count - 4)\n");
    var hits = 0;
    for i in x - 1.. {
        if i == 3 { break }
        hits = hits + 1
    }
    print("\(hits == 3)\n");
}
