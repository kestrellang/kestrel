// test: diagnostics
// stdlib: false

module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[lang.i64]) -> lang.i64 {
    match opt {
        .Some(a, b) => lang.i64_add(a, b), // ERROR
        .None => 0
    }
}
