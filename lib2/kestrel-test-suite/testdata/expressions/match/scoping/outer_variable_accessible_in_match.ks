// test: diagnostics
// stdlib: false

module Main

func test(b: lang.i1) -> lang.i64 {
    let multiplier: lang.i64 = 10;
    match b {
        true => lang.i64_mul(multiplier, 2),
        false => multiplier
    }
}
