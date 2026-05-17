// test: diagnostics
// stdlib: false
module Test
struct Inner {
    var value: lang.i64
}

struct Outer {
    var inner: Inner
}

func makeOuter() -> Outer {
    Outer(inner: Inner(value: 42))
}
