// test: diagnostics
// stdlib: false

module Test
struct Outer {
    struct Inner {}
}
@dummy(Outer.Inner)
struct Foo {}
