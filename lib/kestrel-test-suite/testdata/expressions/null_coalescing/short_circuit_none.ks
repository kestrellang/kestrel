// test: diagnostics

module Main
import std.io.stdio.println

func getDefault() -> Int {
     println("DEFAULT");
    99
}

func main() -> lang.i64 {
    let a: Int? = null;
    let result = a ?? getDefault();
     println(result);
    0
}
