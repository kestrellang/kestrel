// test: execution
// stdlib: true
// expect-stdout: 55 89\ny x\n7 seven\n2 1\n3 1\n1 2 4\n

// `(a, b) = (b, a + b)` evaluates the whole right side before writing any
// target, so swaps and Fibonacci steps need no temporary. Targets can be
// locals, fields or subscripts; nested tuples destructure; `_` discards.
module Test

struct P { var x: Int64; var y: Int64 }

func pair() -> (Int64, String) { (7, "seven") }

@main
func main() -> Int64 {
    var a = 0;
    var b = 1;
    var i = 0;
    while i < 10 { (a, b) = (b, a + b); i += 1 }
    println("\(a) \(b)");
    var s = "x";
    var t = "y";
    (s, t) = (t, s);
    println("\(s) \(t)");
    var n = 0;
    (n, s) = pair();
    println("\(n) \(s)");
    var p = P(x: 1, y: 2);
    (p.x, p.y) = (p.y, p.x);
    println("\(p.x) \(p.y)");
    var xs = [1, 2, 3];
    (xs(0), xs(2)) = (xs(2), xs(0));
    println("\(xs(0)) \(xs(2))");
    var c = 0;
    ((a, b), c) = ((1, 2), 3);
    (_, c) = (99, 4);
    println("\(a) \(b) \(c)");
    0
}
