// test: execution
// stdlib: true

module Test

struct Inner {
    let value: std.numeric.Int64
}

struct Outer {
    let inner: Inner
    let extra: std.numeric.Int64
}

func make_outer(v: std.numeric.Int64, e: std.numeric.Int64) -> Outer {
    Outer(inner: Inner(value: v), extra: e)
}

@main
func main() -> lang.i64 {
    let o = make_outer(40, 2);
    if o.inner.value + o.extra != 42 { return 1 }
    0
}
