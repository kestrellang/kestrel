// test: diagnostics
// stdlib: false

module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(a: Option[lang.i64], b: Option[lang.i64]) -> lang.i64 {
    guard let .Some(x) = a, let .Some(y) = b, lang.i64_signed_lt(x, y) else {
        return 0
    }
    lang.i64_sub(y, x)
}
