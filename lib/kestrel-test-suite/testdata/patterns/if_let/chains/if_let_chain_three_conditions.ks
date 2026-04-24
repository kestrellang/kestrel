// test: diagnostics
// stdlib: false

module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(a: Option[lang.i64], b: Option[lang.i64], c: Option[lang.i64]) -> lang.i64 {
    if let .Some(x) = a, let .Some(y) = b, let .Some(z) = c {
        lang.i64_add(lang.i64_add(x, y), z)
    } else {
        0
    }
}
