// test: diagnostics
// stdlib: false

module Main

enum Option[T] {
    case Some(value: T)
    case None
}

struct Iterator {
    var current: lang.i64
    var max: lang.i64
}

func next(iter: Iterator) -> Option[lang.i64] {
    if lang.i64_signed_lt(iter.current, iter.max) {
        Option[lang.i64].Some(value: iter.current)
    } else {
        Option[lang.i64].None
    }
}

func test() {
    var iter = Iterator(current: 0, max: 10);
    while let .Some(item) = next(iter) {
        iter.current = lang.i64_add(iter.current, 1);
    }
}
