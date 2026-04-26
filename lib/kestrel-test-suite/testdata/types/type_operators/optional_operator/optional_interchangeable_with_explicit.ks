// test: diagnostics
// stdlib: true

module Test
import std.io.stdio.println

func takeExplicit(x: std.result.Optional[std.num.Int64]) -> std.num.Int64 {
    x.unwrapOr(0)
}

func main() -> lang.i64 {
    let val: std.num.Int64? = .Some(42);
    let _ = println(takeExplicit(val));
    0
}
