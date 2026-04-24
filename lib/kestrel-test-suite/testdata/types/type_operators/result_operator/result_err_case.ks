// test: diagnostics
// stdlib: true

module Test
import std.io.stdio.println

struct MyError {}

func main() -> lang.i64 {
    let err: std.num.Int64 throws MyError = .Err(MyError());
    let _ = println(err.isErr());
    let _ = println(err.unwrapOr(99));
    0
}
