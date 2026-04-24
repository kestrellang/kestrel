// test: diagnostics

module Main
import std.io.stdio.println

func expensiveCheck() -> Bool {
    let _ = println("EXPENSIVE");
    true
}

func main() -> lang.i64 {
    let result = 5 > 10 and expensiveCheck();
    let _ = println(result);
    0
}
