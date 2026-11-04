// test: execution
// stdlib: true
// expect-exit: 0

// An array can't hold StringSlices (they borrow the string), so
// `split(...).collect()` copies each segment into an owned String. The
// result outlives the source string.

module Test

import std.collections.(Array)
import std.text.(String)

func parts() -> Array[String] {
    let line: String = "a,bb,ccc";
    line.split(",").collect()
}

@main
func main() -> lang.i64 {
    let ps = parts();
    if ps.count != 3 { return 1 }
    if ps(0) != "a" { return 2 }
    if ps(2) != "ccc" { return 3 }
    0
}
