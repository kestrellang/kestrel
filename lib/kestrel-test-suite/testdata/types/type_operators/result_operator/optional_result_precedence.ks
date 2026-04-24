// test: diagnostics
// stdlib: true
// skip: blocked on type alias normalization

module Test
import std.io.stdio.println

struct MyError {}

func main() -> lang.i64 {
    let someOk: std.num.Int64 throws MyError? = .Some(.Ok(42));
    let none: std.num.Int64 throws MyError? = .None;
    let _ = println(someOk.unwrap().unwrap());
    let _ = println(none.isNone());
    0
}
