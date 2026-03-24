// test: diagnostics
// stdlib: false

module Test
enum Optional[T]: Prelude.ExpressibleByNullLiteral {
    case Some(T)
    case None

    init() {
        self = Optional.None
    }
}
func test() -> Optional[lang.i64] {
    null
}
