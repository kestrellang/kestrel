// test: diagnostics

module Main
import std.streams.println

func effectC() -> Bool {
     println("C");
    false
}

func main() -> lang.i64 {
    let result = false or true or effectC();
     println(result);
    0
}
