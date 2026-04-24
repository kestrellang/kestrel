// test: diagnostics
// stdlib: false

module Main

enum Option {
    case Some(value: lang.i64)
    case None
}

func makeNone() -> Option {
    Option.None
}
