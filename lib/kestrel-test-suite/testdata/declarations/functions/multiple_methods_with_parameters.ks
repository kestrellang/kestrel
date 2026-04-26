// test: diagnostics
// stdlib: false

module Test

struct Calculator {
    func add(a: lang.i64, b: lang.i64) -> lang.i64 { lang.i64_add(a, b) }
    func subtract(a: lang.i64, b: lang.i64) -> lang.i64 { lang.i64_sub(a, b) }
    func multiply(a: lang.i64, b: lang.i64) -> lang.i64 { lang.i64_mul(a, b) }
}
