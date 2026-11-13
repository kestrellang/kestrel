// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 7
//
// A derived `Cloneable` bounds each type parameter by the `Copyable` lang
// item itself, not by the name `Copyable`: a user type of that name in the
// deriving module does not hijack the bound, so `Box[Int64]` still clones.

module Main

import std.numeric.(Int64)

struct Copyable {}

@derive(Cloneable)
struct Box[T] {
    var value: T
}

func duplicate[T](x: T) -> T where T: Cloneable { x.clone() }

@main
func main() -> Int64 {
    duplicate(Box(value: 7)).value
}
