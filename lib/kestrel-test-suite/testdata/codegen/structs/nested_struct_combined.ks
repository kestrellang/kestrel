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

@main
func main() -> lang.i64 {
    let o = Outer(inner: Inner(value: 40), extra: 2);
    if o.inner.value + o.extra != 42 { return 1 }
    0
}
