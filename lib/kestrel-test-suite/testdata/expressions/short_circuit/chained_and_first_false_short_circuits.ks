// test: diagnostics

module Main
import std.io.stdio.println

func effectB() -> Bool {
    let _ = println("B");
    true
}

func effectC() -> Bool {
    let _ = println("C");
    true
}

func main() -> lang.i64 {
    let result = false and effectB() and effectC();
    let _ = println(result);
    0
}
