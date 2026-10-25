// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0
//
// Derived witnesses only borrow fields, so a `not Copyable` type derives
// like any other (design §C8). Interpolation needs `Copyable`, so the
// formatting check calls `formatted` directly.

module Main

import std.numeric.(Int64)

@derive(Equatable, Hashable, Comparable, Formattable)
struct Resource: not Copyable {
    var id: Int64
    var name: String
}

@main
func main() -> Int64 {
    let a = Resource(id: 1, name: "r");
    let b = Resource(id: 1, name: "r");
    let c = Resource(id: 2, name: "r");
    if a != b { return 1 }
    if a >= c { return 2 }
    if a.formatted(FormatOptions.default()) != "Resource(id: 1, name: r)" { return 3 }
    0
}
