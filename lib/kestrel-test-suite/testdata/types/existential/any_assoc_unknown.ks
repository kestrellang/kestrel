// test: diagnostics
// stdlib: true
//
// E150: an associated-type binding must name an associated type of the
// protocol.

module Test

import std.numeric.(Int64)
import std.iter.(Iterator)

func f(xs: any Iterator[Itm = Int64]) {} // ERROR: 'Iterator' has no associated type named 'Itm'
