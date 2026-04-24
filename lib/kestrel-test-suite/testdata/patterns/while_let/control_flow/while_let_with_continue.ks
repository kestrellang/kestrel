// test: diagnostics
// stdlib: false

module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func getOption(n: lang.i64) -> Option[lang.i64] {
    if lang.i64_signed_gt(n, 0) {
        Option[lang.i64].Some(value: n)
    } else {
        Option[lang.i64].None
    }
}

func test() -> lang.i64 {
    var sum: lang.i64 = 0;
    var n: lang.i64 = 10;
    while let .Some(value) = getOption(n) {
        n = lang.i64_sub(n, 1);
        if lang.i64_eq(value, 5) {
            continue
        }
        sum = lang.i64_add(sum, value);
    }
    sum
}
