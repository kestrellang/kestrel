// test: diagnostics
// stdlib: true
//
// A derived `Cloneable` copies every field, so each field must be `Copyable`
// (E952). It refines `Copyable`, so a `not Copyable` type can't derive it
// (E954), and a hand-written `clone()` conflicts with the derived one (E953).

module Main

import std.numeric.(Int64)

struct Handle: not Copyable {
    var fd: Int64
}

@derive(Cloneable) // ERROR: cannot derive 'Cloneable' for 'Owner': field 'handle' has type 'Handle', which is not 'Copyable'
struct Owner {
    var handle: Handle
}

@derive(Cloneable) // ERROR: cannot derive 'Cloneable' for 'Resource': it opts out of 'Copyable'
struct Resource: not Copyable {
    var id: Int64
}

@derive(Cloneable) // ERROR: 'Manual' derives 'Cloneable' but also implements 'clone'; remove one
struct Manual {
    var id: Int64
    func clone() -> Self { Manual(id: self.id) }
}
