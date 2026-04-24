// test: diagnostics

module Main
import std.io.stdio.println

func isZero(n: Int) -> Bool {
    n == 0
}

func isNegative(n: Int) -> Bool {
    n < 0
}

func main() -> lang.i64 {
    let _ = println(isZero(0) or isNegative(0));   // true or false (short-circuits)
    let _ = println(isZero(5) or isNegative(-3));  // false or true
    0
}
