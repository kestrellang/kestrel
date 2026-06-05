// test: diagnostics
// stdlib: true

module Test
import std.io.stdio.println

func main() -> lang.i64 {
    var dict: [std.numeric.Int64: std.numeric.Int64] = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64]();
     dict.insert(42, 123);
    let value: std.numeric.Int64 = dict(unwrap: 42);
     println(value);
    0
}
