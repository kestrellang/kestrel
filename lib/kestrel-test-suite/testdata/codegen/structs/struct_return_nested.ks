// test: execution
// stdlib: true

module Test

struct Inner {
    let value: std.num.Int64
}

struct Outer {
    let inner: Inner
    let extra: std.num.Int64
}

func make_outer(v: std.num.Int64, e: std.num.Int64) -> Outer {
    Outer(inner: Inner(value: v), extra: e)
}

func main() -> lang.i64 {
    let o = make_outer(40, 2);
    if o.inner.value + o.extra != 42 { return 1 }
    0
}
