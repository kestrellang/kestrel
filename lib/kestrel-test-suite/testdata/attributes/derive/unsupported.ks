// test: diagnostics
// stdlib: true
//
// E954: `@derive` on an extension needs the type's fields, so the type must
// be declared in this module (and, for 0.18, in the same file). A type with
// an unsized tail can't be derived. Protocols take no `@derive` (E900).

module Main

import std.numeric.(Int64)

@derive(Equatable) // ERROR: '@derive' on an extension needs a type declared in this module
extend Int64 { }

@derive(Equatable) // ERROR: cannot derive 'Equatable' for 'Tail': its last field is unsized
struct Tail[T] where T: not Sized {
    var n: Int64
    var t: T
}

@derive(Equatable) // ERROR: '@derive' cannot be applied to a protocol; it applies to structs, enums and extensions
protocol Marker { }

@derive(Equatable)
@derive(Hashable) // ERROR: '@derive' appears twice; list every protocol in one '@derive(...)'
struct Twice {
    var n: Int64
}

@derive() // ERROR: '@derive' needs a protocol to derive: '@derive(Equatable)'
struct Nothing {
    var n: Int64
}
