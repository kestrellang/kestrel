// test: diagnostics

module Main
import std.io.stdio.println

func expensiveCheck() -> Bool {
     println("EXPENSIVE");
    true
}

func main() -> lang.i64 {
    let result = 5 > 10 and expensiveCheck();
     println(result);
    0
}
