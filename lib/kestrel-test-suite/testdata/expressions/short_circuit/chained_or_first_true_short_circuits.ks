// test: diagnostics

module Main
import std.streams.println

func effectB() -> Bool {
     println("B");
    false
}

func effectC() -> Bool {
     println("C");
    false
}

func main() -> lang.i64 {
    let result = true or effectB() or effectC();
     println(result);
    0
}
