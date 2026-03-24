// test: diagnostics
// stdlib: false

module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(a: Option[lang.i64], b: Option[lang.i64]) -> lang.i64 {
    if let .Some(x) = a {
        x
    } else if let .Some(y) = b {
        y
    } else {
        0
    }
}
