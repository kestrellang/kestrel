// test: execution
// stdlib: true

// Generic function with opaque return: wrap[T] -> some Printable.
// Different instantiations produce different opaque types.

module Test

protocol Printable {
    func value() -> std.numeric.Int64
}

struct Wrapper[T] {
    let inner: T
    public init(inner inner: T) { self.inner = inner }
}

extend Wrapper[std.numeric.Int64]: Printable {
    public func value() -> std.numeric.Int64 { self.inner }
}

func wrap(v: std.numeric.Int64) -> some Printable {
    Wrapper[std.numeric.Int64](inner: v)
}

@main
func main() -> lang.i64 {
    let a = wrap(42);
    if a.value() != 42 { return 1 }
    let b = wrap(99);
    if b.value() != 99 { return 2 }
    0
}
