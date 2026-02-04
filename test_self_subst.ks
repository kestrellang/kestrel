module Test

import std.num.(Int64)
import std.collections.(Array)

func main() -> Int64 {
    var arr = Array[Int64]();
    arr.append(1);
    arr.append(2);

    let iter = arr.iter();
    let mapped = iter.map({ (x) in x * 2 });

    0
}
