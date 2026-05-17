// test: diagnostics
// stdlib: false

module Test

struct Inner {
    var x: lang.i64
}

struct Outer {
    var inner: Inner
    var y: lang.i64
}
