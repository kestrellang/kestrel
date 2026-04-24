// test: diagnostics
// stdlib: false

module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[Option[lang.i64]]) -> lang.i64 {
    if let .Some(inner) = opt {
        if let .Some(value) = inner {
            value
        } else {
            0
        }
    } else {
        0
    }
}
