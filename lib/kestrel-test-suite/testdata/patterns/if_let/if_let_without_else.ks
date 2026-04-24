// test: diagnostics
// stdlib: false

module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opt: Option[lang.i64]) {
    if let .Some(value) = opt {
        let _ = value;
    }
}
