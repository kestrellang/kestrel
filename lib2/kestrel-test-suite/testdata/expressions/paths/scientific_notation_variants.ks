// test: diagnostics
// stdlib: false

module Test

func test() {
    1.5e10;
    1.0e-10;
    1.0e+10;
    1.0E10;
    lang.f64_neg(1.0e10);
    lang.f64_neg(1.0e-10);
}
