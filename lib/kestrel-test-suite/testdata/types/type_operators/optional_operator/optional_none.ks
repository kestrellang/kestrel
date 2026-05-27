// test: diagnostics
// stdlib: true

module Test
import std.io.stdio.println

func main() -> lang.i64 {
    let none: std.numeric.Int64? = .None;
    let _ = println(none.unwrap(or: 99));
    0
}
