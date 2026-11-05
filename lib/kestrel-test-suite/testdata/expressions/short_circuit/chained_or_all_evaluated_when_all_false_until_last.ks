// test: diagnostics

module Main
import std.streams.println

func effectA() -> Bool {
     println("A");
    false
}

func effectB() -> Bool {
     println("B");
    false
}

func effectC() -> Bool {
     println("C");
    true
}

func main() -> lang.i64 {
    let result = effectA() or effectB() or effectC();
     println(result);
    0
}
