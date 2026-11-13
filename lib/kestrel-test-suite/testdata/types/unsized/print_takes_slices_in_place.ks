// test: execution
// stdlib: true
// expect-stdout: hi|hi|[1, 2]|ab\n

// `print`, `println` and interpolation are declared `not Sized`, so they
// format a `&StringSlice` or a `&Slice[T]` in place, through a `let`, a
// borrowed parameter, or a tuple element.

module Test

import std.numeric.(Int64)
import std.collections.(Array, Slice)

func showSlice(s: &Slice[Int64]) { print(s) }

@main
func main() {
    let s = "  hi  ";
    let t = s.trimmed();
    print(t);
    print("|\(t)|");
    let xs: Array[Int64] = [1, 2, 3];
    showSlice(xs(0..<2));
    let pair = ("ab".asSlice(), 1);
    println("|\(pair.0)");
}
