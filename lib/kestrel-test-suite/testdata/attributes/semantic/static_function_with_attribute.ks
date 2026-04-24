// test: diagnostics
// stdlib: false

module Test
struct Foo {
    @dummy
    static func create() -> Foo {
        Foo()
    }
}
