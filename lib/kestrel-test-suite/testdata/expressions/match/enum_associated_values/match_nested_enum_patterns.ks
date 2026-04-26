// test: diagnostics
// stdlib: false

module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[Option[lang.i64]]) -> lang.i64 {
    match opt {
        .Some(value: .Some(inner)) => inner,
        .Some(value: .None) => 0,
        .None => 0
    }
}
