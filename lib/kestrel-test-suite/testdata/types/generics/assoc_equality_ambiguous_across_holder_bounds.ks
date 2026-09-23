// test: diagnostics
// stdlib: true

// G25 step 3: `Item.Out = Int64` names `Out`, which `Item` gets only from this
// same where clause, and two of those bounds (`HasOutA`, `HasOutB`) each
// declare an `Out`. The equality cannot say which one it pins, so
// `WhereClausesOf` keeps it out (traced under `KESTREL_DEBUG=where-eq` as
// AMBIGUOUS) rather than picking one by name. With the clause out, `outA()`
// returns an unpinned `Item.Out`, and the body is rejected.
//
// The rejection is correct; its location is not ideal. A dedicated
// "ambiguous associated type in equality" diagnostic at the clause would be
// better, and would replace this annotation.

module Test

import std.numeric.Int64

protocol HasOutA { type Out; func outA() -> Out }
protocol HasOutB { type Out; func outB() -> Out }
protocol Source { type Item; func fetch() -> Item }

extend Source where Item: HasOutA, Item: HasOutB, Item.Out = Int64 {
    public func viaA() -> Int64 { self.fetch().outA() } // ERROR: expected Int64 got Item.Out
}
