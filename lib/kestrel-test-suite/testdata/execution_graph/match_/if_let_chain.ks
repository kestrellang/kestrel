// test: diagnostics
// stdlib: false

module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func addIfBothSome(a: Option[lang.i64], b: Option[lang.i64]) -> lang.i64 {
    if let .Some(x) = a, let .Some(y) = b {
        lang.i64_add(x, y)
    } else {
        0
    }
}
