// test: diagnostics
// stdlib: false

module Test

struct Circle {
    var radius: lang.i64

    var diameter: lang.i64 {
        get {
            lang.i64_mul(self.radius, 2)
        }
    }
}
