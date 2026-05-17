// test: execution
// stdlib: true

module Test

struct Inner {
    var value: std.numeric.Int64
}

struct Outer {
    var inner: Inner
}

func setValue(mutating i: Inner, n: std.numeric.Int64) {
    i.value = n;
}

func main() -> lang.i64 {
    var o = Outer(inner: Inner(value: 0));
    setValue(o.inner, 42);
    if o.inner.value != 42 { return 1 }
    0
}
