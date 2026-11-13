// test: diagnostics

module Main
import std.streams.println

func expensiveDefault() -> Int {
     println("RHS");
    999
}

func main() -> lang.i64 {
    let x: Int? = .Some(42);
    let result = x ?? expensiveDefault();
     println(result);
    0
}
