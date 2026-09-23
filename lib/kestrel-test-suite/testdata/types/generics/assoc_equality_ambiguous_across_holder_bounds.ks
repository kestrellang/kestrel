// test: diagnostics
// stdlib: true

// G25 step 3 / G29: `Item.Out = Int64` names `Out`, which `Item` gets only
// from this same where clause, and two of those bounds (`HasOutA`, `HasOutB`)
// each declare an `Out`. The equality cannot say which one it pins, so
// `ExplicitWhereClauses` drops it rather than picking one by name, and
// `GenericsAnalyzer` reports E479 at the clause.
//
// The body's follow-on mismatch is still reported: with the clause gone,
// `outA()` returns an unpinned `Item.Out`. It cannot be suppressed yet, because
// the body emitters look an equality's associated type up again by name, so
// no recovery clause can reach that use (the name-keyed half of G29).

module Test

import std.numeric.Int64

protocol HasOutA { type Out; func outA() -> Out }
protocol HasOutB { type Out; func outB() -> Out }
protocol Source { type Item; func fetch() -> Item }

extend Source where Item: HasOutA, Item: HasOutB, Item.Out = Int64 { // ERROR: associated type 'Out' in where clause is ambiguous: 'Item' is bound by HasOutA and HasOutB, which each declare 'Out'
    public func viaA() -> Int64 { self.fetch().outA() } // ERROR: expected Int64 got Item.Out
}
