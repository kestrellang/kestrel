// test: execution
// stdlib: true
// expect-exit: 0

// #182 (order-independence): the CONSTRAINED extension is declared FIRST here.
// Selection must still be most-specific-wins — `Box[Int64]` -> `showy`,
// `Box[String]` -> `generic` — identical to the opposite order in
// overlapping_conformance_where_clause.ks. Without binding each witness to its
// own extension's method, this order would mis-bind the unconstrained witness
// to `showy` and a `Box[String]` would wrongly print `showy`.

module Test

import std.numeric.Int64
import std.text.String

protocol Show { func show() -> String }
extend Int64: Show { public func show() -> String { "i" } }

protocol Tag { func tag() -> String }

struct Box[T] { var value: T; }

extend Box[T]: Tag where T: Show { public func tag() -> String { "showy" } }
extend Box[T]: Tag { public func tag() -> String { "generic" } }

@main
func main() -> lang.i32 {
    if Box(value: 7).tag() != "showy" { return 10 }       // Int64: Show
    if Box(value: "x").tag() != "generic" { return 11 }   // String !: Show
    0
}
