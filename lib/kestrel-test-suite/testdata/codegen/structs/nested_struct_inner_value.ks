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

func main() -> lang.i64 {
    let o = Outer(inner: Inner(value: 42), extra: 0);
    if o.inner.value != 42 { return 1 }
    0
}
