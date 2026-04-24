// test: diagnostics
// stdlib: false

module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test() -> lang.i64 {
    var count: lang.i64 = 0;
    var opt: Option[lang.i64] = .Some(value: 100);
    while let .Some(value) = opt {
        count = lang.i64_add(count, 1);
        if lang.i64_signed_gt(count, 5) {
            break
        }
        opt = .Some(value: lang.i64_sub(value, 1));
    }
    count
}
