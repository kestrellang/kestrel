// test: diagnostics
// stdlib: true

module Test
import std.io.stdio.println

func main() -> lang.i64 {
    var dict: [std.numeric.Int64: std.numeric.Int64] = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64]();
     dict.insert(1, 100);
     dict.insert(2, 200);
     println(dict.count);
    0
}
