// test: execution
// stdlib: false
// expect-exit: 9

// The motivating case behind shipping Exitable's `Result[(), E]` conformance
// alone (cf. issue #110 / the Result[T,E]+Result[(),E] overlap). The generic
// conformance is guarded by `where T: Marked`; a separate specialized
// conformance covers `Wrap[lang.i64]` even though lang.i64 is NOT Marked.
//
// For `Wrap[lang.i64]` the *only* logically-valid conformance is the
// specialized one (9): the generic's where-clause `lang.i64: Marked` is
// unsatisfied. But the witness selector matches purely structurally and ignores
// the where-clause, so it currently routes through the generic body — whose
// `self.value.mark()` then needs `lang.i64: Marked`, which does not exist ->
// the `Callee::Witness not resolved` overlap ICE. Correct behavior: select the
// specialized conformance -> 9.
module Main

protocol Marked { func mark() -> lang.i64 }

struct Wrap[T] { var value: T }

extend Wrap[T]: Marked where T: Marked { func mark() -> lang.i64 { self.value.mark() } }
extend Wrap[lang.i64]: Marked          { func mark() -> lang.i64 { 9 } }

func markOf[T](x: T) -> lang.i64 where T: Marked { x.mark() }

@main
func main() -> lang.i64 {
    let w = Wrap[lang.i64](value: 0);
    markOf(w) // specialized conformance is the only valid one -> 9
}
