// test: diagnostics
// stdlib: false
module Test
enum Option[T] {
    case Some(value: T)
    case None
}

func test() {
    let x = Option[lang.i64].None;
}
