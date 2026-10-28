// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0
//
// Derived bounds (design §C3): a type parameter gets `T: P` when a field's
// type mentions it, else no bound. `Box[T]: Equatable where T: Equatable`;
// the phantom `Tagged[Tag]` is `Equatable` for every `Tag`.

module Main

import std.numeric.(Int64)

struct NotEquatable { var v: Int64 }

@derive(Equatable, Hashable)
struct Box[T] {
    var value: T
    var history: Array[T]
}

@derive(Equatable)
struct Tagged[Tag] {
    var id: Int64
}

@derive(Equatable)
enum Either[L, R] {
    case Left(L)
    case Right(R)
}

@main
func main() -> Int64 {
    let a = Box(value: 1, history: [1, 2]);
    if a != Box(value: 1, history: [1, 2]) { return 1 }
    if a == Box(value: 1, history: [2]) { return 2 }
    if Box(value: "s", history: []) != Box(value: "s", history: []) { return 3 }
    let t1: Tagged[NotEquatable] = Tagged(id: 1);
    let t2: Tagged[NotEquatable] = Tagged(id: 1);
    if t1 != t2 { return 5 }
    let l: Either[Int64, String] = Either.Left(3);
    if l != Either.Left(3) { return 6 }
    if l == Either.Right("3") { return 7 }
    0
}
