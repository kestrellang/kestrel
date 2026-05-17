// test: diagnostics
// stdlib: false

module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func unwrapOr[T](opt: Option[T], default: T) -> T {
    match opt {
        .Some(value) => value,
        .None => default
    }
}
