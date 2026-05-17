// test: diagnostics
// stdlib: false

module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[lang.i64]) -> lang.i64 {
    guard let .Some(x) = opt, lang.i64_signed_gt(x, 0) else {
        return 0
    }
    lang.i64_mul(x, 2)
}
