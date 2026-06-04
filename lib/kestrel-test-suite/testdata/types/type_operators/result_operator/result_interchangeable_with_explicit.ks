// test: diagnostics
// stdlib: true

module Test
import std.io.stdio.println

struct MyError {}

func handleExplicit(r: std.result.Result[std.numeric.Int64, MyError]) -> std.numeric.Int64 {
    r.unwrap(or: 0)
}

func main() -> lang.i64 {
    let val: std.numeric.Int64 throws MyError = .Ok(42);
    let _ = println(handleExplicit(val));
    0
}
