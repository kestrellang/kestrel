// test: diagnostics
// stdlib: false

module Main

struct Inner {
    let value: lang.i64
}

struct Outer {
    let inner: Inner
}

func getDeep(o: Outer) -> lang.i64 {
    o
        .inner
        .value
}
