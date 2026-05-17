// test: diagnostics
// stdlib: false

module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[lang.i64]) -> lang.i1 {
    match opt {
        .Some(_) => true,
        .None => false
    }
}
