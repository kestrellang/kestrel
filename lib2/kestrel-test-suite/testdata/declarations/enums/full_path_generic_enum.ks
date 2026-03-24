// test: diagnostics
// stdlib: false

module Test

enum Option[T] {
    case Some(value: T)
    case None
}

func getSome() -> Option[lang.i64] {
    Option.Some(value: 42)
}
