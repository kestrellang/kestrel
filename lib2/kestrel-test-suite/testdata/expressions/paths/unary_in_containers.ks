// test: diagnostics

module Test

func test() {
    [lang.i64_neg(1), lang.i64_neg(2), lang.i64_neg(3)];
    (lang.i64_neg(1), lang.i64_neg(2));
    [([(lang.i64_neg(1),)],)];
}
