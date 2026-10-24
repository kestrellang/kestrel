// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0
//
// A recursive enum derives through a container: `List: Equatable` needs
// `Array[List]: Equatable`, which needs `List: Equatable` again (design §C8).

module Main

import std.numeric.(Int64)

@derive(Equatable)
enum List {
    case Nil
    case Cons(Int64, Array[List])
}

@main
func main() -> Int64 {
    let a = List.Cons(1, [List.Cons(2, [List.Nil])]);
    let b = List.Cons(1, [List.Cons(2, [List.Nil])]);
    let c = List.Cons(1, [List.Nil]);
    if a != b { return 1 }
    if a == c { return 2 }
    0
}
