// test: diagnostics
// stdlib: true

module Test
import std.io.stdio.println

func countExplicit(dict: std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64]) -> std.numeric.Int64 {
    dict.count
}

func main() -> lang.i64 {
    var dict: [std.numeric.Int64: std.numeric.Int64] = std.collections.Dictionary[std.numeric.Int64, std.numeric.Int64]();
     dict.insert(1, 1);
     println(countExplicit(dict));
    0
}
