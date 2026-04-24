// test: diagnostics

module Main
import std.io.stdio.println

func effectB() -> Bool {
    let _ = println("B");
    false
}

func effectC() -> Bool {
    let _ = println("C");
    false
}

func main() -> lang.i64 {
    let result = true or effectB() or effectC();
    let _ = println(result);
    0
}
