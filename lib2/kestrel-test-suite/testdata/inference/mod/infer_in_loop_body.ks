// test: diagnostics
// stdlib: false

module Main

func test() -> lang.i64 {
    var sum = 0;
    var i = 0;
    loop {
        if lang.i64_signed_ge(i, 10) {
            break
        }
        let x = lang.i64_mul(i, i);
        sum = lang.i64_add(sum, x);
        i = lang.i64_add(i, 1);
    }
    sum
}
