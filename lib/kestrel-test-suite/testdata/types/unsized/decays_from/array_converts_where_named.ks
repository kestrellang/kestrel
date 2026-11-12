// test: execution
// stdlib: true
// expect-stdout: 2\n1\n3\n

// A borrowed `&Slice[T]` converts to an owned `Array[T]` wherever the
// target names `Array` (`DecaysFrom[Slice[T]]`), including a subscript whose
// result type is only known once its index type is (`I.SliceOutput`).

module Test

import std.numeric.(Int64)
import std.collections.(Array)

func head(xs: Array[Int64]) -> Array[Int64] { xs(0..<1) }

func total(consuming xs: Array[Int64]) -> Int64 { xs.count }

@main
func main() {
    let n: Array[Int64] = [1, 2, 3];
    let lo: Array[Int64] = n(0..<2);
    println("\(lo.count)");
    println("\(head(n).count)");
    println("\(total(n(0..<3)))");
}
