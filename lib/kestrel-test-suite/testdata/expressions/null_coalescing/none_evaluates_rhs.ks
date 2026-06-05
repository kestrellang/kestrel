// test: diagnostics

module Main
import std.io.stdio.println

func expensiveDefault() -> Int {
     println("RHS");
    999
}

func main() -> lang.i64 {
    let x: Int? = null;
    let result = x ?? expensiveDefault();
     println(result);
    0
}
