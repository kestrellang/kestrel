// test: diagnostics
// stdlib: true
//
// A recursive case must use the enum's own type parameters: `Nested[[T]]`
// inside `Nested[T]` would need a new instantiation at every level.

module Test

import std.collections.(Array)

indirect enum Nested[T] {
    case Flat(T)
    case Nest(Nested[Array[T]]) // ERROR: indirect enum 'Nested' contains itself with different type arguments
}
