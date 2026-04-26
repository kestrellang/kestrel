// test: diagnostics
// stdlib: false

module Main
func calculate(x: lang.i64, y: lang.i64) -> lang.i64 {
    let sum = lang.i64_add(x, y);
    let product = lang.i64_mul(x, y);
    lang.i64_add(sum, product)
}
