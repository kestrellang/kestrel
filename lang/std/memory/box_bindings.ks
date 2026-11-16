// Box bindings — which box each compiler feature instantiates.
//
// The compiler never names a box type. Each boxing feature reads one tagged
// alias below, follows it to a struct, and touches that struct only through
// the protocol requirements in `box.ks` (docs/design/shared-box.md). Std picks
// the boxes here, in one place:
//
// - a ROLE alias (`Shared`, `Owned`) names the box std uses for that kind of
//   ownership. Swapping the role's box retargets every feature on it.
// - a FEATURE alias names the box one compiler feature uses. It normally
//   points at a role; pointing one at a different box splits that feature
//   off without touching the others.
//
// Each feature tag carries the protocol bound the compiler needs from its
// box, checked where the alias is declared (E470).

module std.memory

/// The stdlib's shared box: copies of a handle alias one value, and the last
/// handle to go frees it. `RcBox` promises deterministic last-release cleanup
/// and does not collect strong cycles.
public type Shared[T] = RcBox[T]

/// The stdlib's single-owner box: one handle owns the value and may give it
/// back. Also the box E145/E148 suggest when an existential needs to consume
/// its value (`any[OwnedBox] P`).
@builtin(.OwnedBox)
public type Owned[T] = OwnedBox[T]

/// Escaping closure environments. Needs `SharedBox`: aliases of a closure
/// share and mutate its captured state.
@builtin(.ClosureBox)
public type ClosureBox[T] = Shared[T]

/// Class storage. Needs `SharedBox`: class values are handles, `===` is
/// `isIdentical(to:)`. Unused until classes land.
@builtin(.ClassBox)
public type ClassBox[T] = Shared[T]

/// The box plain `any P` is written in: `any P` is `AnyBox[<raw any P>]`.
/// Needs `Box`. Write `any[B] P` to pick a box at one site.
@builtin(.AnyBox)
public type AnyBox[T] = Shared[T]

/// The boxed payload of a recursive `indirect enum` case. Needs `Box` and
/// `ConsumingIndirection`: matching a case moves the payload back out.
@builtin(.IndirectBox)
public type IndirectBox[T] = Owned[T]

/// `consuming` closure environments. Needs `Box` and `MutableIndirection`:
/// the environment is boxed as `Optional[E]`, the one call swaps it out for
/// `.None` in place, and the box's ordinary drop frees what is left, so a
/// closure that is never called still drops its captures.
@builtin(.ConsumingClosureBox)
public type ConsumingClosureBox[T] = Owned[T]
