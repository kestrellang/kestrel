// test: diagnostics
// stdlib: true

module Main

enum Option[T] {
    case Some(value: T)
    case None
}

func test() {
    var arr = std.collections.Array[Option[std.numeric.Int64]]();
    for .Some(x) in arr { // ERROR: refutable
        ()
    }
}
