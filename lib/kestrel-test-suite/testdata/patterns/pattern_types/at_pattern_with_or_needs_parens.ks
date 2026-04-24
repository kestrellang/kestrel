// test: diagnostics
// stdlib: false

module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[lang.i64]) -> Option[lang.i64] {
    match opt {
        x @ (.Some(_) or .None) => x
    }
}
