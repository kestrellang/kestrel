// test: diagnostics

module Main
import std.io.stdio.println

func main() -> lang.i64 {
    var i: Int = 0;
    var sum: Int = 0;
    while i < 5 and sum < 6 {
        sum = sum + i;
        i = i + 1;
    }
    let _ = println(i);
    let _ = println(sum);
    0
}
