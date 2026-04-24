// test: diagnostics

module Main
import std.io.stdio.println

func effectA() -> Bool {
    let _ = println("A");
    false
}

func effectB() -> Bool {
    let _ = println("B");
    false
}

func effectC() -> Bool {
    let _ = println("C");
    true
}

func main() -> lang.i64 {
    let result = effectA() or effectB() or effectC();
    let _ = println(result);
    0
}
