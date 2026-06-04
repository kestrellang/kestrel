// test: execution
// stdlib: true

module Test

import std.numeric.(Int64)
import std.collections.(Array)

// A closure literal with no `mutating` annotation, passed where a
// `(mutating Array[Int64]) -> Int64` is expected. Its param convention is
// inferred as MutBorrow (#106), so calling the mutating method `append` on it
// is allowed (no E203) and mutates the array in place — with no leak.
func mutate(mutating arr: Array[Int64], with body: (mutating Array[Int64]) -> Int64) -> Int64 {
    body(arr)
}

@main
func main() -> lang.i64 {
    var arr = Array[Int64]();
    var i = 0;
    while i < 100 {
        let _ = mutate(arr, with: { (a) in a.append(i); a.count });
        i = i + 1;
    }
    if arr.count != 100 { return 1 }
    if arr(99) != 99 { return 2 }
    0
}
