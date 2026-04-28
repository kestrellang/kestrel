// test: diagnostics
// stdlib: true
// skip: blocked on type alias normalization

module Test
import std.io.stdio.println

struct MyError {}

func main() -> lang.i64 {
    let okVal: std.numeric.Int64 throws (MyError?) = .Ok(42);
    let errNone: std.numeric.Int64 throws (MyError?) = .Err(.None);
    let _ = println(okVal.unwrap());
    let _ = println(errNone.isErr());
    0
}
