// test: execution
// stdlib: true
// expect-exit: 0

// Two conditional conformances split on the builtin `Copyable`: mono must
// pick the one whose clause holds for the concrete arg. `Copyable` is
// structural (no witness declares it), so mono used to read `T: Copyable`
// as failing for every `T`, which made `T: not Copyable` hold for every `T`
// and silently ran the `not Copyable` witness for `Box[Int64]` (exit 10).

module Test

import std.numeric.Int64

protocol Name {
    func name() -> Int64
}

struct Mv: not Copyable {
    var x: Int64
}

struct Box[T] where T: not Copyable {
    var v: T
}

extend Box[T]: Name where T: Copyable {
    func name() -> Int64 { 1 }
}

extend Box[T]: Name where T: not Copyable {
    func name() -> Int64 { 2 }
}

func nameOf[N](consuming n: N) -> Int64 where N: Name, N: not Copyable { n.name() }

@main
func main() -> lang.i32 {
    if nameOf(Box[Int64](v: 5)) != 1 { return 10 }
    if nameOf(Box[Mv](v: Mv(x: 1))) != 2 { return 11 }
    0
}
