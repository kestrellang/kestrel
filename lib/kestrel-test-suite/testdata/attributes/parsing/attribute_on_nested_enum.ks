// test: diagnostics
// stdlib: false

module Test
struct Outer {
    @dummy
    enum Inner { case A }
}
