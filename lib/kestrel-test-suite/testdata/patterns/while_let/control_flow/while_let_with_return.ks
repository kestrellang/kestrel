// test: diagnostics
// stdlib: false

module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test() -> lang.i64 {
    var opt: Option[lang.i64] = .Some(value: 42);
    while let .Some(value) = opt {
        if lang.i64_signed_gt(value, 40) {
            return value
        }
        opt = .Some(value: lang.i64_sub(value, 1));
    }
    0
}
