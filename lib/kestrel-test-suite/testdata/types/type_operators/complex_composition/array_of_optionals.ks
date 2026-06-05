// test: diagnostics
// stdlib: true
// skip: blocked on type alias normalization

module Test
import std.io.stdio.println

func main() -> lang.i64 {
    var arr: [std.numeric.Int64?] = std.collections.Array[std.result.Optional[std.numeric.Int64]]();
    arr.append(.Some(1));
    arr.append(.None);
    arr.append(.Some(3));
     println(arr.count);
     println(arr.first().unwrap().unwrap());
    0
}
