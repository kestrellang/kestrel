// test: diagnostics
// stdlib: false

module Test
func f32Ops(a: lang.f32, b: lang.f32) {
    let _add = lang.f32_add(a, b);
    let _sub = lang.f32_sub(a, b);
    let _mul = lang.f32_mul(a, b);
    let _div = lang.f32_div(a, b);
}
