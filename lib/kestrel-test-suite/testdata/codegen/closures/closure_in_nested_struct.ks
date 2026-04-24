// test: execution
// stdlib: true

module Test

struct Inner {
    let transform: (std.num.Int64) -> std.num.Int64
}

struct Outer {
    let inner: Inner
    let value: std.num.Int64
}

func main() -> lang.i64 {
    let outer = Outer(
        inner: Inner(transform: { (x) in x + 12 }),
        value: 30
    );
    if (outer.inner.transform)(outer.value) != 42 { return 1 }
    0
}
