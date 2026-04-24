// test: execution
// stdlib: true

module Test

struct Inner {
    var value: std.num.Int64
}

struct Outer {
    var inner: Inner
    var extra: std.num.Int64
}

func main() -> lang.i64 {
    var o = Outer(inner: Inner(value: 0), extra: 0);
    o.inner.value = 30;
    o.extra = 12;
    if o.inner.value + o.extra != 42 { return 1 }
    0
}
