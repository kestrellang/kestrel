// test: diagnostics
// stdlib: true
// skip: blocked on type alias normalization

module Test
import std.io.stdio.println

struct MyError {}

func main() -> lang.i64 {
    var arr: [std.numeric.Int64 throws MyError] = std.collections.Array[std.result.Result[std.numeric.Int64, MyError]]();
    arr.append(.Ok(1));
    arr.append(.Err(MyError()));
    arr.append(.Ok(3));
     println(arr.count);
     println(arr.first().unwrap().unwrap());
    0
}
