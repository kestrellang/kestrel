// test: diagnostics
// stdlib: true

module Test
import std.io.stdio.println

struct MyError {}

func main() -> lang.i64 {
    let ok: std.numeric.Int64 throws MyError = .Ok(42);
     println(ok.unwrap());
    0
}
