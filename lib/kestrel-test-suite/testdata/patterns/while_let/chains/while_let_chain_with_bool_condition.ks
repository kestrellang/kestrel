// test: diagnostics
// stdlib: false

module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test() {
    var opt: Option[lang.i64] = Option.Some(value: 5);
    while let .Some(x) = opt, lang.i64_signed_gt(x, 0) {
        opt = Option.Some(value: lang.i64_sub(x, 1));
    }
}
