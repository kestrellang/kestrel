// test: diagnostics
// stdlib: true
// skip: blocked on type alias normalization

module Test
import std.io.stdio.println

func main() -> lang.i64 {
    let someOpt: [std.numeric.Int64]? = .Some(std.collections.Array[std.numeric.Int64]());
    let none: [std.numeric.Int64]? = .None;
    let _ = println(someOpt.isSome());
    let _ = println(none.isNone());
    0
}
