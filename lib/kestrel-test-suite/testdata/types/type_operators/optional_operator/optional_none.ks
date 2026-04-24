// test: diagnostics
// stdlib: true

module Test
import std.io.stdio.println

func main() -> lang.i64 {
    let none: std.num.Int64? = .None;
    let _ = println(none.unwrapOr(99));
    0
}
