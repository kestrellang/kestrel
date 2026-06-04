// test: execution
// stdlib: true

// Recursive function with opaque return. The base case pins the
// concrete type; recursive calls see the internal view.

module Test

protocol Numeric {
    func value() -> std.numeric.Int64
}

struct MyInt {
    let v: std.numeric.Int64
    public init(v v: std.numeric.Int64) { self.v = v }
}

extend MyInt: Numeric {
    public func value() -> std.numeric.Int64 { self.v }
}

func compute(n: std.numeric.Int64) -> some Numeric {
    if n == 0 { return MyInt(v: 42) }
    compute(n - 1)
}

@main
func main() -> lang.i64 {
    let r = compute(5);
    if r.value() != 42 { return 1 }
    0
}
