// test: diagnostics
// stdlib: true
// skip: blocked on type alias normalization

module Test
import std.io.stdio.println

func main() -> lang.i64 {
    var outer: [[std.numeric.Int64]] = std.collections.Array[std.collections.Array[std.numeric.Int64]]();
    var inner: [std.numeric.Int64] = std.collections.Array[std.numeric.Int64]();
    inner.append(1);
    inner.append(2);
    outer.append(inner);
     println(outer.count);
     println(outer.first().unwrap().count);
    0
}
