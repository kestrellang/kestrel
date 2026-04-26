// test: diagnostics
// stdlib: false

module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[lang.i1]) -> lang.i64 {
    match opt { // ERROR: exhaustive
        .Some(value: true) => 1,
        .None => 0
    }
}
