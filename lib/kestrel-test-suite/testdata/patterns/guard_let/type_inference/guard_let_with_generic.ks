// test: diagnostics
// stdlib: false

module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func unwrap[T](opt: Option[T], default: T) -> T {
    guard let .Some(value) = opt else {
        return default
    }
    value
}
