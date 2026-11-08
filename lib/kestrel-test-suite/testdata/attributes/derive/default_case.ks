// test: diagnostics
// stdlib: true
//
// E957: an enum derives `Defaultable` only with exactly one `@default` case.
// `@default` is for enum cases only (E900).

module Main

import std.numeric.(Int64)

@derive(Defaultable) // ERROR: cannot derive 'Defaultable' for 'NoDefault': mark exactly one case '@default'
enum NoDefault {
    case A
    case B
}

@derive(Defaultable) // ERROR: cannot derive 'Defaultable' for 'TwoDefaults': mark exactly one case '@default'
enum TwoDefaults {
    @default
    case A
    @default
    case B
}

@derive(Defaultable)
enum OneDefault {
    case A
    @default
    case B(Int64)
}

struct Holder {
    @default // ERROR: '@default' cannot be applied to a stored field; it applies to enum cases
    var n: Int64
}
