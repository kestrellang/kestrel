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

func main() -> lang.i64 {
    let o = Outer(inner: Inner(value: 40), extra: 42);
    if o.extra != 42 { return 1 }
    0
}
