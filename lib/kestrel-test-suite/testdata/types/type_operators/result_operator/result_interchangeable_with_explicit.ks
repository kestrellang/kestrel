// test: diagnostics
// stdlib: true

module Test
import std.io.stdio.println

struct MyError {}

func handleExplicit(r: std.result.Result[std.num.Int64, MyError]) -> std.num.Int64 {
    r.unwrapOr(0)
}

func main() -> lang.i64 {
    let val: std.num.Int64 throws MyError = .Ok(42);
    let _ = println(handleExplicit(val));
    0
}
