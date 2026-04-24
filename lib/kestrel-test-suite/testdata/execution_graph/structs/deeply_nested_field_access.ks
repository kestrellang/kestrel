// test: diagnostics
// stdlib: false

module Main

struct Inner {
    let value: lang.i64
}

struct Middle {
    let inner: Inner
}

struct Outer {
    let middle: Middle
}

func getValue(o: Outer) -> lang.i64 {
    o.middle.inner.value
}
