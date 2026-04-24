// test: diagnostics
// stdlib: false

module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func unwrap(opt: Option[lang.i64]) -> lang.i64 {
    if let .Some(v) = opt {
        v
    } else {
        0
    }
}
