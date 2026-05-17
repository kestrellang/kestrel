// test: diagnostics

module Main
import std.io.stdio.println

func effectC() -> Bool {
    let _ = println("C");
    true
}

func main() -> lang.i64 {
    let result = true and false and effectC();
    let _ = println(result);
    0
}
