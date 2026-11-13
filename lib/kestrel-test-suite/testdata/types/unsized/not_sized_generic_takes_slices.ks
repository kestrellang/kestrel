// test: execution
// stdlib: true
// expect-stdout: hi\n[1, 2]\n5\n

// A generic whose body only borrows its value can be relaxed with
// `not Sized`, so it takes an unsized `StringSlice` or `Slice[T]` in place.
// `not Sized` also drops the implicit Copyable bound: a value that may be
// unsized is never held by value, so it is never copied.

module Test

import std.numeric.(Int64)
import std.collections.(Array)

func show[T](x: &T) where T: Formattable, T: not Static, T: not Sized {
    println(x.formatted());
}

@main
func main() {
    let s = "  hi  ";
    show(s.trimmed());
    let xs: Array[Int64] = [1, 2, 3];
    let a: Int64 = 0;
    let b: Int64 = 2;
    let sl = xs(a..<b);
    show(sl);
    show(5);
}
