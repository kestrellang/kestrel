// test: diagnostics
// stdlib: false

module Test
struct Resource {
    var data: lang.i64

    @dummy
    consuming func take() -> lang.i64 {
        self.data
    }
}
