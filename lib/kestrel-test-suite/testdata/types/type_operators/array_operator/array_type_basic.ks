// test: diagnostics
// stdlib: true
// skip: blocked on type alias normalization

module Test
import std.io.stdio.println

func main() -> lang.i64 {
    var arr: [std.num.Int64] = std.collections.Array[std.num.Int64]();
    arr.append(10);
    arr.append(20);
    arr.append(30);
    let _ = println(arr.count);
    0
}
