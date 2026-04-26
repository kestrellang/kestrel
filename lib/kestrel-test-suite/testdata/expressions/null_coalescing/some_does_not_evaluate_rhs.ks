// test: diagnostics

module Main
import std.io.stdio.println

func expensiveDefault() -> Int {
    let _ = println("RHS");
    999
}

func main() -> lang.i64 {
    let x: Int? = .Some(42);
    let result = x ?? expensiveDefault();
    let _ = println(result);
    0
}
