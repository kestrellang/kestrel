// test: diagnostics

module Test

func test(foo: lang.i64) {
    let x: lang.i64 = lang.i64_neg(foo);
    let y: (lang.i64, lang.i64) = (lang.i64_neg(1), lang.i64_neg(2));
    let z: [lang.i64] = [lang.i64_neg(1), lang.i64_neg(2), lang.i64_neg(3)];
    let w: (lang.i64, lang.i1, lang.i64) = (lang.i64_neg(1), lang.i1_not(true), lang.i64_neg(lang.i64_neg(2)));
}
