// test: diagnostics

module Main
import std.io.stdio.println

func main() -> lang.i64 {
    var a: Int = 0;
    var b: Int = 10;
    var count: Int = 0;
    // Loop while a < 3 OR b > 8 (exits when both are false)
    while a < 3 or b > 8 {
        a = a + 1;
        b = b - 1;
        count = count + 1;
    }
     println(count);  // 3: after 3 iterations, a=3 (not <3) and b=7 (not >8)
    0
}
