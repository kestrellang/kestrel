// test: diagnostics
// stdlib: false

module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test() -> lang.i64 {
    var sum: lang.i64 = 0;
    var current: Option[lang.i64] = .Some(value: 5);
    while let .Some(n) = current {
        sum = lang.i64_add(sum, n);
        if lang.i64_signed_gt(n, 0) {
            current = .Some(value: lang.i64_sub(n, 1));
        } else {
            current = .None;
        }
    }
    sum
}
