// test: diagnostics

module Main
import std.io.stdio.println

func effectA() -> Bool {
    let _ = println("A");
    true
}

func effectB() -> Bool {
    let _ = println("B");
    true
}

func effectC() -> Bool {
    let _ = println("C");
    true
}

func main() -> lang.i64 {
    let result = effectA() and effectB() and effectC();
    let _ = println(result);
    0
}
