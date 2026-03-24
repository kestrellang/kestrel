// test: diagnostics
// stdlib: true

module Test
import std.io.stdio.println

func main() -> lang.i64 {
    var dict: [std.num.Int64: std.num.Int64] = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
    let _ = dict.insert(1, 100);
    let _ = dict.insert(2, 200);
    let _ = println(dict.count);
    0
}
