// test: diagnostics
// stdlib: false

module Test

struct Temperature {
    var celsius: lang.i64

    var fahrenheit: lang.i64 {
        get {
            lang.i64_add(lang.i64_signed_div(lang.i64_mul(self.celsius, 9), 5), 32)
        }
        set {
            self.celsius = lang.i64_signed_div(lang.i64_mul(lang.i64_sub(newValue, 32), 5), 9)
        }
    }
}
