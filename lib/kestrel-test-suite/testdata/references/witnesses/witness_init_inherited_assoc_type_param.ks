// test: execution
// stdlib: true
// expect-exit: 0

// Phase-0 validation for closure-kinds plan D9 (docs/plans/closure-kinds/
// closure-kinds-plan.md) and docs/design/shared-box.md Resolved Q2: "Creation
// is `init(consuming value: Target)` — a protocol init requirement, matching
// `RcBox`'s existing init exactly." `Target` is INHERITED (`SharedBox: Mutable-
// Indirection`), so the requirement's parameter type is an associated type the
// protocol does not itself declare, and the conformer binds it in an
// EXTENSION. If that shape cannot be witnessed, shared-box.md's documented
// fallback spelling is `static func create(consuming value: Target) -> Self`
// — fallback only, not the design. This test therefore validates an EXISTING
// capability with existing language features; it is expected to pass today.
//
// Shape mirrored from declarations/extensions/self_constructor_two_conformers.ks
// (protocol init witnessed through an extension) and
// types/static_type_param/init_from_inherited_protocol.ks (constructing through
// an inherited init requirement on a type parameter).
//
// `value` is a single-name parameter, so the requirement is called
// POSITIONALLY (`H(v)`) — exactly like `RcBox.init(consuming value: T)`, whose
// call sites read `RcBox(fd)`. Witness matching is label-exact, so the
// requirement and the witness must agree on that spelling.
module Test

import std.numeric.(Int64)

protocol Q {
    type Slot
}

protocol P: Q {
    init(consuming value: Slot)
    func read() -> Slot
}

struct Cell[T] {
    var stored: T
}

extend Cell[T]: P {
    type Slot = T

    public init(consuming value: T) { self.stored = value; }

    public func read() -> T { self.stored }
}

// Constructs through the inherited-assoc-type init requirement on `H`.
func make[X, H](v: X) -> H where H: P, H.Slot = X {
    H(v)
}

@main
func main() -> lang.i64 {
    let c: Cell[Int64] = make(7);
    if c.read() != 7 { return 1 }

    let s: Cell[Bool] = make(true);
    if s.read() != true { return 2 }

    0
}
