// test: execution
// stdlib: true
// expect-exit: 0
// expect-stdout: true false false\n

// A conformance argument named bare (`Equal[W]`) means `W` with its
// defaults, as the method's `other: W` does, even when the default is a
// type alias (`DA` for `D`, like `GlobalAllocator` for `SystemAllocator`).
// A generic alias (`Twice[T]`) expands with its args substituted, and an
// alias of an alias expands all the way. The two sides used to differ, so
// the overloads were told apart wrongly and the right `equal` was reported
// as mistyped (E632).
module Test

import std.core.(Equal)

protocol P {}
struct D: P {}
type DA = D

struct W[A = DA] where A: P {
    var a: Int64
}

struct Two[X, Y] { var x: X; var y: Y }
type Twice[T] = Two[T, T]
type IntPair = Twice[Int64]

struct S: Equal[S], Equal[W], Equal[IntPair] {
    type Equal[S].Output = Bool
    type Equal[W].Output = Bool
    type Equal[IntPair].Output = Bool
    func equal(to other: S) -> Bool { true }
    func equal(to other: W) -> Bool { false }
    func equal(to other: Two[Int64, Int64]) -> Bool { false }
}

@main
func main() {
    print("\(S() == S()) \(S() == W(a: 1)) \(S() == Two(x: 1, y: 2))\n");
}
