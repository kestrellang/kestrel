// test: diagnostics
// stdlib: false

module Test
struct Outer {
    @dummy
    struct Inner {
        var x: lang.i64
    }
}
