// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0
//
// `@derive(Hashable)` feeds every field to the hasher in order; an enum
// feeds its case index first, so `.A(1)` and `.B(1)` hash apart. Derived
// keys work in a `Set`.

module Main

import std.numeric.(Int64, UInt64)
import std.collections.(DefaultHasher, Set)

@derive(Equatable, Hashable)
struct Key {
    var id: Int64
    var tag: String
}

@derive(Equatable, Hashable)
enum Slot {
    case A(Int64)
    case B(Int64)
    case Off
}

func hashOf[T](value: T) -> UInt64 where T: Hashable {
    var hasher = DefaultHasher();
    value.hash(into: hasher);
    hasher.finish()
}

@main
func main() -> Int64 {
    if hashOf(Key(id: 1, tag: "x")) != hashOf(Key(id: 1, tag: "x")) { return 1 }
    if hashOf(Key(id: 1, tag: "x")) == hashOf(Key(id: 2, tag: "x")) { return 2 }
    if hashOf(Key(id: 1, tag: "x")) == hashOf(Key(id: 1, tag: "y")) { return 3 }
    if hashOf(Slot.A(1)) == hashOf(Slot.B(1)) { return 4 }
    if hashOf(Slot.A(1)) != hashOf(Slot.A(1)) { return 5 }
    if hashOf(Slot.Off) != hashOf(Slot.Off) { return 6 }
    var keys: Set[Key] = Set();
    keys.insert(Key(id: 1, tag: "x"));
    keys.insert(Key(id: 1, tag: "x"));
    keys.insert(Key(id: 2, tag: "x"));
    if keys.count != 2 { return 7 }
    if keys.contains(Key(id: 2, tag: "x")) == false { return 8 }
    0
}
