// test: execution
// stdlib: true

module Test

struct Inner {
    var value: std.num.Int64
}

struct Outer {
    var inner: Inner
}

func set_inner_value(mutating o: Outer, v: std.num.Int64) {
    o.inner.value = v;
}

func main() -> lang.i64 {
    var o = Outer(inner: Inner(value: 0));
    set_inner_value(o, 42);
    if o.inner.value != 42 { return 1 }
    0
}
