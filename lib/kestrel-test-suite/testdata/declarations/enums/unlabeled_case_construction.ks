// test: diagnostics
// stdlib: false

module Test

enum Option[T] {
    case Some(T)
    case None
}

func wrap(value: lang.i64) -> Option[lang.i64] {
    Option.Some(value)
}
