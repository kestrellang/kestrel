// test: diagnostics
// stdlib: true

module Test
enum Option[T]: BooleanConditional {
    case Some(T)
    case None

    func boolValue() -> lang.i1 {
        match self {
            .Some(_) => true,
            .None => false
        }
    }
}
func test(opt: Option[lang.i64]) -> lang.i64 {
    if opt {
        1
    } else {
        0
    }
}
