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
    let o = Outer(inner: Inner(value: 1));
    o.inner.value = 10; // ERROR: cannot assign to immutable field
    o.inner.value
}
