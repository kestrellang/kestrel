// test: diagnostics
// stdlib: false

module Test

struct Temperature {
    var celsius: lang.i64

    init(celsius celsius: lang.i64) {
        self.celsius = celsius
    }

    init(fahrenheit fahrenheit: lang.i64) {
        self.init(celsius: lang.i64_signed_div(lang.i64_mul(lang.i64_sub(fahrenheit, 32), 5), 9))
    }
}
