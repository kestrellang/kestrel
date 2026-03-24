// test: diagnostics
// stdlib: true

module Test
import std.io.stdio.println

struct MyError {}

func main() -> lang.i64 {
    let ok: std.num.Int64 throws MyError = .Ok(42);
    let _ = println(ok.unwrap());
    0
}
