// test: execution
// stdlib: true
// expect-stdout: a=5 xs0=7 k=9\n

// `u.g(0) = 5` where `g` is a get/set computed property (not a place): the
// property is gotten into a temporary, the subscript writes into it, and the
// setter stores it back. The store used to land in the getter's temporary
// and was dropped (Array and Dictionary receivers crashed).
module Test

struct G {
    var a: Int64
    subscript(i: Int64) -> Int64 { get { self.a } set { self.a = newValue; } }
}

struct U {
    var rg: G
    var xs: [Int64]
    var d: [String: Int64]
    var g: G { get { self.rg } set { self.rg = newValue; } }
    var items: [Int64] { get { self.xs } set { self.xs = newValue; } }
    var dict: [String: Int64] { get { self.d } set { self.d = newValue; } }
}

@main
func main() -> Int64 {
    var u = U(rg: G(a: 0), xs: [0], d: ["k": 0]);
    u.g(0) = 5;
    u.items(0) = 7;
    u.dict(unwrap: "k") = 9;
    print("a=\(u.rg.a) xs0=\(u.xs(0)) k=\(u.d(unwrap: "k"))\n");
    0
}
