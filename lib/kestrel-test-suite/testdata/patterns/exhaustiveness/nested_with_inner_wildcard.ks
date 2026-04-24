// test: diagnostics
// stdlib: false

module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[lang.i1]) -> lang.i64 {
    match opt {
        .Some(value: true) => 1,
        .Some(_) => 2,
        .None => 0
    }
}
