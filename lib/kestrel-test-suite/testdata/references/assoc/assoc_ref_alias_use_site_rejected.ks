// test: diagnostics
// stdlib: true

// Stage 2d anti-smuggling: a ref-valued member alias is POSITION-
// TRANSPARENT. Trivial aliases are eagerly expanded at every named use,
// so an illegal position rejects exactly as if `&T` were written there
// (E480 param / E482 binding). The diagnostic anchors at the alias RHS —
// one illegal use per alias below so each line carries one error.
module Test

struct Holder[T] {
    type Item = &T // ERROR(E480)
    type Slot = &T // ERROR(E482)
    var v: T

    func smuggleParam(x: Item) -> Int64 {
        0
    }

    func smuggleBinding() -> Int64 {
        let r: Slot = self.v;
        0
    }
}
