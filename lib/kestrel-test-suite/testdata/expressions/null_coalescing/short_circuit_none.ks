// test: diagnostics

module Main
import std.io.stdio.println

func getDefault() -> Int {
    let _ = println("DEFAULT");
    99
}

func main() -> lang.i64 {
    let a: Int? = null;
    let result = a ?? getDefault();
    let _ = println(result);
    0
}
