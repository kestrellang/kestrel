// test: execution
// stdlib: true
// expect-exit: 0
// expect-stdout: 1 2 3\n

// A bare extension target takes the declared defaults for its defaulted
// params, as when some args are written (Rust's rule): `extend Tagged` is
// `extend Tagged[Int64]`, so a bare `Tagged` inside it is `Self`. Params
// before the first default stay open: `extend Keyed` covers every `K`.
module Test
import std.streams.println

struct Tagged[T = Int64] {
    var value: T
}

extend Tagged {
    func same() -> Tagged { self }
    func read() -> Int64 { self.value }
}

struct Keyed[K, H = Bool] {
    var key: K
    var flag: H
}

extend Keyed {
    func two() -> Int64 { 2 }
}

@main
func main() -> lang.i64 {
    let t = Tagged(value: 1);
    let a = Keyed(key: "x", flag: true);
    let b = Keyed(key: 3.5, flag: false);
    let x = t.same().read();
    let y = a.two();
    let z = b.two() + 1;
    println("\(x) \(y) \(z)");
    0
}
