// test: diagnostics
// stdlib: true
//
// E952: every field (or enum payload) must conform to the derived protocol.
// The error is reported once, at the `@derive` argument; the generated
// body's own errors are not shown on top of it. `Defaultable` exempts fields
// with an initializer.

module Main

import std.numeric.(Int64)

struct Tag {
    var v: Int64
}

@derive(Equatable)
struct EqualOnly {
    var v: Int64
}

@derive(Equatable, Hashable) // ERROR: cannot derive 'Hashable' for 'Point': field 'tag' has type 'EqualOnly', which is not 'Hashable'
struct Point {
    var x: Int64
    var tag: EqualOnly
}

@derive(Equatable) // ERROR: cannot derive 'Equatable' for 'Shape': payload 'radius' of case 'Circle' has type 'Tag', which is not 'Equatable'
enum Shape {
    case Circle(radius: Tag)
    case Empty
}

@derive(Defaultable) // ERROR: cannot derive 'Defaultable' for 'Settings': field 'limit' has type 'Int64?', which is not 'Defaultable'
struct Settings {
    var limit: Int64?
    var tag: Tag = Tag(v: 1)
}

@derive(Defaultable)
struct Initialized {
    var tag: Tag = Tag(v: 1)
}
