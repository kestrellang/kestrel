// test: diagnostics
// stdlib: true
// skip: blocked on type alias normalization

module Test
import std.io.stdio.println

func main() -> lang.i64 {
    var arr: [std.numeric.Int64] = std.collections.Array[std.numeric.Int64]();
    arr.append(10);
    arr.append(20);
    arr.append(30);
     println(arr.first().unwrap());
     println(arr.last().unwrap());
    0
}
