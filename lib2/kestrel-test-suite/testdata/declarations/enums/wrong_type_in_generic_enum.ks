// test: diagnostics
// stdlib: false

module Test

enum Option[T] {
    case Some(value: T)
    case None
}

func test() -> Option[lang.i64] {
    Option.Some(value: "hello") // ERROR: does not conform to protocol
}
