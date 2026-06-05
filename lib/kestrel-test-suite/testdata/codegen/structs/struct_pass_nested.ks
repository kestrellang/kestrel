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

func sum_outer(o: Outer) -> std.numeric.Int64 {
    o.inner.value + o.extra
}

@main
func main() -> lang.i64 {
    let o = Outer(inner: Inner(value: 30), extra: 12);
    if sum_outer(o) != 42 { return 1 }
    0
}
