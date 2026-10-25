// test: diagnostics
// stdlib: true
//
// E958: a derived protocol's parents must hold: derived alongside it,
// declared, or derived by another `@derive`. Parents are never derived
// implicitly.

module Main

import std.numeric.(Int64)

@derive(Hashable) // ERROR: deriving 'Hashable' requires 'Point' to be 'Equatable'; add 'Equatable' to '@derive'
struct Point {
    var x: Int64
}

@derive(Comparable) // ERROR: deriving 'Comparable' requires 'Version' to be 'Equatable'; add 'Equatable' to '@derive'
struct Version {
    var major: Int64
}

@derive(Hashable, Equatable)
struct Together {
    var x: Int64
}

@derive(Hashable)
struct ByHand: Equatable {
    var x: Int64

    public func isEqual(to other: ByHand) -> Bool { self.x == other.x }
}

struct Split {
    var x: Int64
}

@derive(Equatable)
extend Split { }

@derive(Hashable)
extend Split { }
