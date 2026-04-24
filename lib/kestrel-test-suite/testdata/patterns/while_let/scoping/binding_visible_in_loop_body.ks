// test: diagnostics
// stdlib: false

module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test() -> lang.i64 {
    var result = 0;
    var opt: Option[lang.i64] = .Some(value: 42);
    while let .Some(value) = opt {
        result = value;
        opt = .None;
    }
    result
}
