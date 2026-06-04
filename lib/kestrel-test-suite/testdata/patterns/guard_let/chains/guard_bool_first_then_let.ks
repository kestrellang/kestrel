// test: diagnostics
// stdlib: false

module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(flag: lang.i1, opt: Option[lang.i64]) -> lang.i64 {
    guard flag, let .Some(x) = opt else {
        return 0
    }
    lang.i64_mul(x, 2)
}
