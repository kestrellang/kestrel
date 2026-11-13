// test: execution
// stdlib: true
// expect-stdout: 0 1 5 1\n[0, 1, 1, 2, 3]\n
//
// When every field has a default, the memberwise init takes no arguments:
// `Fib()` works without a hand-written `init() {}`, also on a type that
// conforms to a protocol. (The stdlib review hit this as an error; it now
// works, and this pins it.)

module Test

struct Fib {
    var a: Int64 = 0
    var b: Int64 = 1
}

struct Fibonacci: Iterator {
    type Item = Int64
    var a: Int64 = 0
    var b: Int64 = 1

    mutating func next() -> Int64? {
        let out = self.a;
        let sum = self.a + self.b;
        self.a = self.b;
        self.b = sum;
        .Some(out)
    }
}

@main
func main() {
    let f = Fib();
    let g = Fib(a: 5);
    println("\(f.a) \(f.b) \(g.a) \(g.b)");
    println(Fibonacci().take(5).collect());
}
