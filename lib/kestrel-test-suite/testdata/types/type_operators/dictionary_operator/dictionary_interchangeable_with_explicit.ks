// test: diagnostics
// stdlib: true

module Test
import std.io.stdio.println

func countExplicit(dict: std.collections.Dictionary[std.num.Int64, std.num.Int64]) -> std.num.Int64 {
    dict.count
}

func main() -> lang.i64 {
    var dict: [std.num.Int64: std.num.Int64] = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
    let _ = dict.insert(1, 1);
    let _ = println(countExplicit(dict));
    0
}
