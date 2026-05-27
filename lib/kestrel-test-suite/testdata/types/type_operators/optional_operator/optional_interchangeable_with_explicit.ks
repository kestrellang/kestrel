// test: diagnostics
// stdlib: true

module Test
import std.io.stdio.println

func takeExplicit(x: std.result.Optional[std.numeric.Int64]) -> std.numeric.Int64 {
    x.unwrap(or: 0)
}

func main() -> lang.i64 {
    let val: std.numeric.Int64? = .Some(42);
    let _ = println(takeExplicit(val));
    0
}
