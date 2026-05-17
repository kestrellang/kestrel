// test: diagnostics
// stdlib: false

module Main

enum Option {
    case Some(value: lang.i64)
    case None
}

func makeSome(x: lang.i64) -> Option {
    Option.Some(value: x)
}
