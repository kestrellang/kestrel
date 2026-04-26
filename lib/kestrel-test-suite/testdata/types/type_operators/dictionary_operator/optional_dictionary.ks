// test: diagnostics
// stdlib: true
// skip: blocked on type alias normalization

module Test
import std.io.stdio.println

func main() -> lang.i64 {
    let some: [std.num.Int64: std.num.Int64]? = .Some(std.collections.Dictionary[std.num.Int64, std.num.Int64]());
    let none: [std.num.Int64: std.num.Int64]? = .None;
    let _ = println(some.isSome());
    let _ = println(none.isNone());
    0
}
