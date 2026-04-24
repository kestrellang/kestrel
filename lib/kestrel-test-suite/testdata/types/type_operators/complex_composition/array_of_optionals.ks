// test: diagnostics
// stdlib: true
// skip: blocked on type alias normalization

module Test
import std.io.stdio.println

func main() -> lang.i64 {
    var arr: [std.num.Int64?] = std.collections.Array[std.result.Optional[std.num.Int64]]();
    arr.append(.Some(1));
    arr.append(.None);
    arr.append(.Some(3));
    let _ = println(arr.count);
    let _ = println(arr.first().unwrap().unwrap());
    0
}
