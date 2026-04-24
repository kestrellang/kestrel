// test: diagnostics
// stdlib: true
// skip: blocked on type alias normalization

module Test
import std.io.stdio.println

func main() -> lang.i64 {
    var dict: [std.num.Int64: [std.num.Int64]] = std.collections.Dictionary[std.num.Int64, std.collections.Array[std.num.Int64]](0, std.collections.Array[std.num.Int64]());
    var arr: [std.num.Int64] = std.collections.Array[std.num.Int64]();
    arr.append(10);
    arr.append(20);
    let _ = dict.insert(1, arr);
    let _ = println(dict.getValue(1).unwrap().count);
    0
}
