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
extend Option[T]: Prelude.FromResidual[NoneEarly] {
    static func fromResidual(residual: NoneEarly) -> Option[T] {
        Option.None
    }
}
func addToOption(opt: Option[lang.i64], value: lang.i64) -> Option[lang.i64] {
    Option.Some(lang.i64_add(try opt, value))
}
