// test: diagnostics
// stdlib: true
// skip: blocked on type alias normalization

module Test
import std.io.stdio.println

struct MyError {}

func main() -> lang.i64 {
    var arr: [std.num.Int64 throws MyError] = std.collections.Array[std.result.Result[std.num.Int64, MyError]]();
    arr.append(.Ok(1));
    arr.append(.Err(MyError()));
    arr.append(.Ok(3));
    let _ = println(arr.count);
    let _ = println(arr.first().unwrap().unwrap());
    0
}
