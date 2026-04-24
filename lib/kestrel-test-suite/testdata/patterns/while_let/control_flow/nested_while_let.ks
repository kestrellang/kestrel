// test: diagnostics
// stdlib: false

module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test() -> lang.i64 {
    var sum: lang.i64 = 0;
    var outer: Option[lang.i64] = .Some(value: 3);
    while let .Some(i) = outer {
        var inner: Option[lang.i64] = .Some(value: i);
        while let .Some(j) = inner {
            sum = lang.i64_add(sum, j);
            if lang.i64_signed_gt(j, 0) {
                inner = .Some(value: lang.i64_sub(j, 1));
            } else {
                inner = .None;
            }
        }
        if lang.i64_signed_gt(i, 0) {
            outer = .Some(value: lang.i64_sub(i, 1));
        } else {
            outer = .None;
        }
    }
    sum
}
