// test: diagnostics
// stdlib: true

// docs/design/shared-box.md: `SharedBox` is a real protocol usable as a generic
// bound ("the compiler codes against the protocol, never a concrete type"). A
// plain struct supplies none of the requirements — no `init(consuming:)`, no
// `sharedMutRef`, no `isIdentical`/`isUnique`, and no Cloneable /
// MutableIndirection refinement — so it must be rejected at the bound.
module Test

import std.memory.(SharedBox)
import std.numeric.(Int64)
import std.core.(Bool)

struct Plain {
    var v: Int64
}

func needsBox[B](box: B) -> Bool where B: SharedBox {
    box.isUnique()
}

func test() -> Bool {
    needsBox(Plain(v: 1)) // ERROR: does not conform to protocol
}
