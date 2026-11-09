// test: execution
// stdlib: true
// expect-stdout: 1\n2\n

// Two same-label inits on a generic type, one of them generic itself: the
// pick by argument type still passes the type's own args (A) to the init,
// ahead of the init's I. Missing A failed post-mono verification.

module Test
protocol Dflt { init() }
struct Al: Dflt { init() {} }
struct S[A = Al] where A: Dflt {
    var n: Int64
    init(n: Int64) { self.n = n; }
}
extend S[A] where A: Dflt {
    init[I](from x: I) where I: Iterable, I.Item = Int64 { self = S[A](1); }
}
extend S[A] where A: Dflt {
    init(from x: Bool) { self = S[A](2); }
}
@main
func main() {
    let a = S(from: [5]);
    println(a.n);
    let b = S(from: true);
    println(b.n);
}
