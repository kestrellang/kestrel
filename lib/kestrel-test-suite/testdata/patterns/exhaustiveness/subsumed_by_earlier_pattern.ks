// test: diagnostics
// stdlib: false

module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[lang.i64]) -> lang.i64 {
    match opt {
        .Some(_) => 1,
        .Some(value: 42) => 2, // WARN: unreachable
        .None => 0
    }
}
