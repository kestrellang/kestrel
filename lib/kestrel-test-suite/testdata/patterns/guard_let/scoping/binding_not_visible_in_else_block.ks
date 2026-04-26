// test: diagnostics
// stdlib: false

module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[lang.i64]) -> lang.i64 {
    guard let .Some(value) = opt else {
        return value // ERROR: undefined
    }
    value
}
