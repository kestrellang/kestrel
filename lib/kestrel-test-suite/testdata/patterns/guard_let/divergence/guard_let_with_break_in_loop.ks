// test: diagnostics
// stdlib: true

module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test(opts: [Option[lang.i64]]) -> lang.i64 {
    var sum: lang.i64 = 0;
    var i: lang.i64 = 0;
    while lang.i64_signed_lt(i, 10) {
        guard let .Some(value) = Option.Some(value: i) else {
            break
        }
        sum = lang.i64_add(sum, value);
        i = lang.i64_add(i, 1);
    }
    sum
}
