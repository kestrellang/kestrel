// test: diagnostics
// stdlib: false

module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test() {
    var a: Option[lang.i64] = Option.Some(value: 1);
    var b: Option[lang.i64] = Option.Some(value: 2);
    while let .Some(x) = a, let .Some(y) = b {
         lang.i64_add(x, y);
        a = Option[lang.i64].None;
    }
}
