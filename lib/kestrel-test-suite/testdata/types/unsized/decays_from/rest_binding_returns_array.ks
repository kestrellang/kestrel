// test: execution
// stdlib: true
// expect-stdout: 2 2\n

// An array pattern's rest binding is a `&Slice[T]` view; returning it where
// the return type names `Array[T]` converts it to an owned array.

module Main

import std.numeric.(Int64)

func tail(arr: [Int64]) -> [Int64] {
    match arr {
        [_, ..rest] => rest,
        [] => []
    }
}

@main
func main() {
    let r = tail([1, 2, 3]);
    println("\(r.count) \(r(0))");
}
