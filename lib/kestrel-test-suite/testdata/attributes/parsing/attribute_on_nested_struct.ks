// test: diagnostics
// stdlib: false

module Test
struct Outer {
    @dummy
    struct Inner {}
}
