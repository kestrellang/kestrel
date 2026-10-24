// test: diagnostics
// stdlib: true
//
// E950: every `@derive` argument names a protocol. E951: that protocol has a
// built-in deriver. `Copyable` and `Cloneable` are never derived: they follow
// from the fields (design §C1).

module Main

import std.numeric.(Int64)

protocol Shiny {
    func shine() -> Int64
}

struct NotAProtocol {
    var v: Int64
}

@derive(Equtable) // ERROR: cannot find protocol 'Equtable'
struct Typo {
    var x: Int64
}

@derive(NotAProtocol) // ERROR: 'NotAProtocol' in '@derive' is not a protocol
struct Wrong {
    var x: Int64
}

@derive(Shiny) // ERROR: 'Shiny' cannot be derived; the derivable protocols are Equatable, Hashable, Comparable, Formattable and Defaultable
struct Bright {
    var x: Int64
}

@derive(Cloneable) // ERROR: 'Cloneable' is not derived: whether 'Doc' is Cloneable follows from its fields
struct Doc {
    var x: Int64
}

@derive(Copyable) // ERROR: 'Copyable' is not derived: whether 'Plain' is Copyable follows from its fields
struct Plain {
    var x: Int64
}

@derive(Equatable, Equatable) // ERROR: 'Equatable' is listed twice in '@derive'
struct Twice {
    var x: Int64
}
