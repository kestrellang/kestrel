// test: diagnostics
// stdlib: false

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
func add(a: Option[lang.i64], b: Option[lang.i64]) -> Option[lang.i64] {
    let x = try a;
    let y = try b;
    Option.Some(lang.i64_add(x, y))
}
