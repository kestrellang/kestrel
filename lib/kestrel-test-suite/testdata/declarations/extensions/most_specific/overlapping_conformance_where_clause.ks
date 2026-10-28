// test: execution
// stdlib: true
// expect-exit: 0

// #182: two overlapping conformances of a generic type to the same protocol —
// one unconstrained, one with a `where` clause — must resolve most-specific-
// wins, NOT first-declared. `extend Box[T]: Tag where T: Show` is more specific
// than `extend Box[T]: Tag`, so a `Box[Int64]` (Int64: Show) picks `showy`
// while a `Box[String]` (String !: Show) falls back to `generic`. The selection
// must be the same regardless of which extension is declared first.
//
// This file declares the UNCONSTRAINED extension first (the order that used to
// pick `generic` for everything). See overlapping_conformance_where_clause_reordered.ks
// for the opposite declaration order.

module Test

import std.numeric.Int64
import std.text.String

protocol Show { func show() -> String }
extend Int64: Show { public func show() -> String { "i" } }

protocol Tag { func tag() -> String }

struct Box[T] { var value: T; }

extend Box[T]: Tag { public func tag() -> String { "generic" } }
extend Box[T]: Tag where T: Show { public func tag() -> String { "showy" } }

// A generic call site: the witness is dispatched at mono with T = Box[...].
func describe[T](x: T) -> String where T: Tag { x.tag() }

@main
func main() -> lang.i32 {
    // Direct calls.
    if Box(value: 7).tag() != "showy" { return 10 }       // Int64: Show
    if Box(value: "x").tag() != "generic" { return 11 }   // String !: Show

    // Through a generic `where T: Tag` bound (same witness dispatch).
    if describe(Box(value: 7)) != "showy" { return 20 }
    if describe(Box(value: "x")) != "generic" { return 21 }
    0
}
