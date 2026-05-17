// test: diagnostics
// stdlib: false

module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test() -> lang.i64 {
    var sum: lang.i64 = 0;
    var i: lang.i64 = 0;
    while lang.i64_signed_lt(i, 10) {
        i = lang.i64_add(i, 1);
        // Skip odd numbers using guard-let with continue
        guard let .Some(value) = if lang.i64_eq(lang.i64_signed_rem(i, 2), 0) { Option[lang.i64].Some(value: i) } else { Option[lang.i64].None } else {
            continue
        }
        sum = lang.i64_add(sum, value);
    }
    sum
}
