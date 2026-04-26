// test: diagnostics
// stdlib: false

module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test() -> lang.i64 {
    var sum: lang.i64 = 0;
    let multiplier: lang.i64 = 2;
    var opt: Option[lang.i64] = .Some(value: 5);
    while let .Some(value) = opt {
        sum = lang.i64_add(sum, lang.i64_mul(value, multiplier));
        if lang.i64_signed_gt(value, 0) {
            opt = .Some(value: lang.i64_sub(value, 1));
        } else {
            opt = .None;
        }
    }
    sum
}
