// test: diagnostics

module Main
import std.io.stdio.println

func effectC() -> Bool {
     println("C");
    true
}

func main() -> lang.i64 {
    let result = true and false and effectC();
     println(result);
    0
}
