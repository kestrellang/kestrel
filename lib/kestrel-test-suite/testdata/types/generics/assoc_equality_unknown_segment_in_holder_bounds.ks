// test: diagnostics
// stdlib: true

// G29: an equality clause whose associated type no bound declares. `Item`
// has no declared bounds, and neither bound in the where clause (`HasOutA`)
// declares `Nope`, so `ExplicitWhereClauses` drops the clause. It used to
// vanish without a word; `GenericsAnalyzer` now reports E440 at the clause,
// the code a bound subject (`Item.Nope: P`) already gets. The function form
// has no body that uses the clause, so the E440 is its only diagnostic.

module Test

import std.numeric.Int64

protocol HasOutA { type Out; func outA() -> Out }
protocol Source { type Item; func fetch() -> Item }

extend Source where Item: HasOutA, Item.Nope = Int64 { // ERROR: no associated type 'Nope' on 'Item'
    public func first() -> Item { self.fetch() }
}

func pick[T](x: T) -> T where T: HasOutA, T.Nope = Int64 { x } // ERROR: no associated type 'Nope' on 'T'
