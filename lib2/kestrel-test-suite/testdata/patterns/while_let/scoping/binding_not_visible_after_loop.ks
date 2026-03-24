// test: diagnostics
// stdlib: false

module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test() -> lang.i64 {
    var opt: Option[lang.i64] = .None;
    while let .Some(value) = opt {
        opt = .None;
    }
    value // ERROR: undefined
}
