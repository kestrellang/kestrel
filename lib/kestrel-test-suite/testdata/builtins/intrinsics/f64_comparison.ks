// test: diagnostics
// stdlib: false

module Test
func comparison() {
    let _eq = lang.f64_eq(1.0, 1.0);
    let _ne = lang.f64_ne(1.0, 2.0);
    let _lt = lang.f64_lt(1.0, 2.0);
    let _gt = lang.f64_gt(2.0, 1.0);
    let _le = lang.f64_le(1.0, 1.0);
    let _ge = lang.f64_ge(1.0, 1.0);
}
