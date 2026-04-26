// test: diagnostics
// stdlib: true

module Test
import std.io.stdio.println

func main() -> lang.i64 {
    var dict: [std.num.Int64: std.num.Int64] = std.collections.Dictionary[std.num.Int64, std.num.Int64]();
    let _ = dict.insert(42, 123);
    let value: std.num.Int64 = dict(unwrap: 42);
    let _ = println(value);
    0
}
