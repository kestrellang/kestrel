// test: execution
// stdlib: true
// expect-exit: 0

// A conditional conformance gated on the builtin `Copyable` must be found by
// mono, not only by the front end. `Copyable` is structural: no witness ever
// declares it, so mono used to read `T: Copyable` as "no source supplies it",
// reject the only witness, and fail post-mono verification with
//   does not implement 'name' required by 'Test.Name'
// The `where T: Equatable` form of the same program always worked.

module Test

import std.numeric.Int64

protocol Name {
    func name() -> Int64
}

struct Box[T] {
    var v: T
}

extend Box[T]: Name where T: Copyable {
    func name() -> Int64 { 1 }
}

// Reaching the witness through a generic bound forces mono to select it.
func nameOf[N](n: N) -> Int64 where N: Name { n.name() }

@main
func main() -> lang.i32 {
    if nameOf(Box[Int64](v: 5)) != 1 { return 10 }
    0
}
