// test: diagnostics

module Main
import std.io.stdio.println

func effectC() -> Bool {
    let _ = println("C");
    false
}

func main() -> lang.i64 {
    let result = false or true or effectC();
    let _ = println(result);
    0
}
