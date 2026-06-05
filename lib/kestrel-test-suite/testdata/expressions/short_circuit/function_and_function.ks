// test: diagnostics

module Main
import std.io.stdio.println

func isPositive(n: Int) -> Bool {
    n > 0
}

func isEven(n: Int) -> Bool {
    n % 2 == 0
}

func main() -> lang.i64 {
     println(isPositive(4) and isEven(4));  // true and true
     println(isPositive(-1) and isEven(4)); // false and true (short-circuits)
    0
}
