// test: diagnostics
// stdlib: true
//
// Recursion directly, through a tuple, a struct, `Optional`, or another
// indirect enum is accepted once the enum is `indirect`; a plain enum
// holding an indirect enum is fine too (the indirect enum is its own box).

module Test

import std.numeric.(Int64)

indirect enum List {
    case Nil
    case Cons(Int64, List)
}

indirect enum Pairs {
    case End
    case More((Int64, Pairs))
}

struct Forest {
    var trees: Optional[Tree]
}

indirect enum Tree {
    case Leaf
    case Node(Int64, Forest)
}

enum Holder {
    case Empty
    case Full(List)
}
