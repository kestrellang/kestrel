// test: diagnostics
// stdlib: false

module Test

func test() {
    lang.i64_neg(42);
    lang.f64_neg(3.14);
    lang.i1_not(true);
    lang.i1_not(lang.i1_not(false));
    lang.i64_neg(lang.i64_neg(42));
}
