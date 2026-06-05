// test: diagnostics

module Main
import std.io.stdio.println

func effectA() -> Bool {
     println("A");
    true
}

func effectB() -> Bool {
     println("B");
    true
}

func effectC() -> Bool {
     println("C");
    true
}

func main() -> lang.i64 {
    let result = effectA() and effectB() and effectC();
     println(result);
    0
}
