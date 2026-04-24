// test: diagnostics
// stdlib: false

module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[lang.i64]) -> lang.str {
    match opt {
        .Some(value: 1 or 2 or 3) => "small",
        .Some(_) => "large",
        .None => "nothing"
    }
}
