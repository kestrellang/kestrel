// test: diagnostics
// stdlib: false

module Test
struct Inner {
    var value: lang.i64
}
struct Outer {
    var inner: Inner
}
func test() -> lang.i64 {
    var o = Outer(inner: Inner(value: 1));
    o.inner.value = 10;
    o.inner.value
}
