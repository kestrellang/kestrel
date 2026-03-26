// test: diagnostics
// stdlib: false
// include: try_prelude.ks

module Test
enum Option[T] {
    case Some(T)
    case None
}
struct NoneEarly {}
extend Option[T]: Prelude.Tryable {
    type Output = T
    type Early = NoneEarly

    func tryExtract() -> Prelude.ControlFlow[T, NoneEarly] {
        match self {
            .Some(v) => Prelude.ControlFlow.Continue(v),
            .None => Prelude.ControlFlow.Break(NoneEarly())
        }
    }
}
// Missing FromResidual conformance, so try cannot convert the early value
func maybeValue(opt: Option[lang.i64]) -> lang.i64 {
    try opt // ERROR: fromResidual
}
