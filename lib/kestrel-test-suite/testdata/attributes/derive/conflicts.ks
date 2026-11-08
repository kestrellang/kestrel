// test: diagnostics
// stdlib: true
//
// E953: a derived protocol can't also be declared by hand, and no member
// written for the type may implement one of its requirements.

module Main

import std.numeric.(Int64)

@derive(Equatable) // ERROR: 'Listed' already declares 'Equatable'
struct Listed: Equatable {
    var x: Int64
}

@derive(Equatable) // ERROR: 'Extended' already declares 'Equatable'
struct Extended {
    var x: Int64
}

extend Extended: Equatable {
    public func isEqual(to other: Extended) -> Bool { true }
}

@derive(Equatable) // ERROR: 'Written' derives 'Equatable' but also implements 'isEqual'; remove one
struct Written {
    var x: Int64

    public func isEqual(to other: Written) -> Bool { true }
}

@derive(Comparable, Equatable) // ERROR: 'Elsewhere' derives 'Comparable' but also implements 'compare'; remove one
struct Elsewhere {
    var x: Int64
}

extend Elsewhere {
    public func compare(other: Elsewhere) -> Ordering { Ordering.Equal }
}
