// test: diagnostics
// stdlib: true
// skip: blocked on type alias normalization

module Test
import std.io.stdio.println

func countExplicit(arr: std.collections.Array[std.num.Int64]) -> std.num.Int64 {
    arr.count
}

func main() -> lang.i64 {
    var arr: [std.num.Int64] = std.collections.Array[std.num.Int64]();
    arr.append(1);
    arr.append(2);
    let _ = println(countExplicit(arr));
    0
}
