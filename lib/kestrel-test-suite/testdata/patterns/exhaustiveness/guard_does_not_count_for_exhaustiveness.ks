// test: diagnostics
// stdlib: false

module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[lang.i64]) -> lang.str {
    match opt { // ERROR: exhaustive
        .Some(n) if lang.i64_signed_gt(n, 0) => "positive",
        .None => "nothing"
    }
}
