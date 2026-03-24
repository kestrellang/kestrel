// test: diagnostics
// stdlib: false

module Test

func test(x: lang.i64, b: lang.i1) {
    lang.i64_neg(x);
    lang.i1_not(b);
    lang.i64_neg(lang.i64_not(lang.i64_neg(lang.i64_not(x))));
}
