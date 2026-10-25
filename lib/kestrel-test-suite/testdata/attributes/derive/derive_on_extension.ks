// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0
//
// `@derive` on an extension uses that extension's target and `where` clause
// as written, with no inferred bounds: `Handle[T]` is `Equatable` for every
// `T`, because only its `raw` field is compared.

module Main

import std.numeric.(Int64)

struct NotEquatable { var v: Int64 }

struct Handle[T] {
    var raw: Int64
}

@derive(Equatable, Hashable)
extend Handle[T] { }

@main
func main() -> Int64 {
    let a: Handle[NotEquatable] = Handle(raw: 1);
    let b: Handle[NotEquatable] = Handle(raw: 1);
    let c: Handle[NotEquatable] = Handle(raw: 2);
    if a != b { return 1 }
    if a == c { return 2 }
    0
}
