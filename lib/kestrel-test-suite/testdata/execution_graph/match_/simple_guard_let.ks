// test: diagnostics
// stdlib: false

module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func process(opt: Option[lang.i64]) -> lang.i64 {
    guard let .Some(v) = opt else {
        return 0
    }
    lang.i64_mul(v, 2)
}
