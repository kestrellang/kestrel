// test: diagnostics
// stdlib: true

// G25 step 3 / G29: `Item.Out = Int64` names `Out`, which `Item` gets only
// from this same where clause, and two of those bounds (`HasOutA`, `HasOutB`)
// each declare an `Out`. The equality cannot say which one it pins, so
// `ExplicitWhereClauses` drops it rather than picking one by name, and
// `GenericsAnalyzer` reports E479 at the clause.
//
// E479 is the only error. The dropped clause is replaced by error-typed
// stand-ins (`Item.<HasOutA.Out>` and `Item.<HasOutB.Out>` pinned to the error
// type), so the body's `outA()` absorbs instead of reporting a follow-on
// mismatch. That works because the body emitters key an equality by its
// associated-type entity, not its name (G29).

module Test

import std.numeric.Int64

protocol HasOutA { type Out; func outA() -> Out }
protocol HasOutB { type Out; func outB() -> Out }
protocol Source { type Item; func fetch() -> Item }

extend Source where Item: HasOutA, Item: HasOutB, Item.Out = Int64 { // ERROR: associated type 'Out' in where clause is ambiguous: 'Item' is bound by HasOutA and HasOutB, which each declare 'Out'
    public func viaA() -> Int64 { self.fetch().outA() }
}
