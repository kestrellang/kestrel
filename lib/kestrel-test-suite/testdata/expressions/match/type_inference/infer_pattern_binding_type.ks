// test: diagnostics
// stdlib: false

module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[lang.i64]) -> lang.i64 {
    match opt {
        .Some(x) => lang.i64_add(x, 1),
        .None => 0
    }
}
