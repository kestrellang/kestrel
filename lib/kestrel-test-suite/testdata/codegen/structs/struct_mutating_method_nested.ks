// test: execution
// stdlib: true

module Test

struct Inner {
    var value: std.numeric.Int64

    mutating func setValue(to: std.numeric.Int64) {
        self.value = to;
    }
}

struct Outer {
    var inner: Inner
}

func main() -> lang.i64 {
    var o = Outer(inner: Inner(value: 0));
    // Call mutating method directly on var inner
    var inner = o.inner;
    inner.setValue(42);
    o.inner = inner;
    if o.inner.value != 42 { return 1 }
    0
}
