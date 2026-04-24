// test: diagnostics
// stdlib: false

module Test

struct Inner {
    let value: lang.i64
}

struct Outer {
    let inner: Inner
    let extra: lang.i64
}

func main() -> lang.i64 {
    let o = Outer(inner: Inner(value: 10), extra: 5);
    lang.i64_add(o.inner.value, o.extra)
}
