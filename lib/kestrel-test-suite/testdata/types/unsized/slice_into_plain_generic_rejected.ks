// test: diagnostics
// stdlib: true

// A plain generic param is implicitly Sized (and Copyable, and Static), so
// it can't take a slice by value, and a `&StringSlice` isn't Static either.
// The error's detail says it is unsized and how to relax the param.

module Test

import std.text.(String)

func keep[T](x: T) -> Int64 { 1 }

func use(s: String) -> Int64 {
    keep(s.trimmed()) // ERROR(E107)
}
