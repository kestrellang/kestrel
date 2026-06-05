// test: diagnostics
// stdlib: true
// skip: blocked on type alias normalization

module Test
import std.io.stdio.println

func main() -> lang.i64 {
    let nested: std.numeric.Int64?? = .Some(.Some(42));
     println(nested.unwrap().unwrap());
    0
}
