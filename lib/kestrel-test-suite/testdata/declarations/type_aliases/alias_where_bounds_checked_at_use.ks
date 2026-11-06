// test: diagnostics

// An alias's `where` bounds are checked wherever it is used. They used to be
// ignored entirely: `Keyed[NotEq]` was accepted.

module Test

struct NotEq { var v: Int64 }

type Keyed[K] where K: Equatable = Array[K];

func ok(x: Keyed[Int64]) -> Int64 { x.count }

func generic[T](x: Keyed[T]) -> Int64 where T: Equatable { x.count }

func bad(x: Keyed[NotEq]) -> Int64 { x.count } // ERROR: NotEq !: Equatable

func genericBad[T](x: Keyed[T]) -> Int64 { x.count } // ERROR: T !: Equatable
