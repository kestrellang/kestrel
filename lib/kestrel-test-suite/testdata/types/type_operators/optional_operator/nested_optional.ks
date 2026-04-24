// test: diagnostics
// stdlib: true
// skip: blocked on type alias normalization

module Test
import std.io.stdio.println

func main() -> lang.i64 {
    let nested: std.num.Int64?? = .Some(.Some(42));
    let _ = println(nested.unwrap().unwrap());
    0
}
