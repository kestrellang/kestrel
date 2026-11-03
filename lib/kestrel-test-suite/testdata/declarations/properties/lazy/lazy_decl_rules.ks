// test: diagnostics
// stdlib: true

// `lazy` needs an instance `var` of a concrete type with a type and an
// initializer (E639), and reading one mutates `self`, so it needs a
// mutable base (E203).
module Test

import std.numeric.(Int64)

struct S {
    lazy let a: Int64 = 1 // ERROR(E639): a `lazy` property must be a `var`
    lazy static var b: Int64 = 1 // ERROR(E639): `lazy` is only supported on instance properties of a struct or enum
    lazy var c: Int64 // ERROR(E639): a `lazy` property needs an initializer
    lazy var d: Int64 { 1 } // ERROR(E639): a `lazy` property can't have accessors
    lazy var e: Int64 = 2
}

lazy var global: Int64 = 3 // ERROR(E639): `lazy` is only supported on instance properties of a struct or enum

func read(s: S) -> Int64 {
    s.e // ERROR(E203)
}
