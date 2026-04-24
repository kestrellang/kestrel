// test: diagnostics
// stdlib: false

module Test

struct Rectangle {
    var width: lang.i64
    var height: lang.i64

    var area: lang.i64 {
        lang.i64_mul(self.width, self.height)
    }
}
