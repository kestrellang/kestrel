// test: diagnostics
// stdlib: false

module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test() {
    var a: Option[lang.i64] = Option.Some(value: 10);
    var b: Option[lang.i64] = Option.Some(value: 5);
    while let .Some(x) = a, let .Some(y) = b, lang.i64_signed_gt(x, y) {
        let _ = lang.i64_sub(x, y);
        a = Option.Some(value: lang.i64_sub(x, 1));
    }
}
