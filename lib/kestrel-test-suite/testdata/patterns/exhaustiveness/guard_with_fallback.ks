// test: diagnostics
// stdlib: false

module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[lang.i64]) -> lang.str {
    match opt {
        .Some(n) if lang.i64_signed_gt(n, 0) => "positive",
        .Some(_) => "non-positive",
        .None => "nothing"
    }
}
