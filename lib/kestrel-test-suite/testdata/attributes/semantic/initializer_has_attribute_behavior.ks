// test: diagnostics
// stdlib: false

module Test
struct Foo {
    var x: lang.i64
    @dummy
    init(x: lang.i64) {
        self.x = x;
    }
}
