// test: diagnostics

module Main
import std.io.stdio.println

func findValue(key: Int) -> Int? {
    if key == 1 {
        .Some(100)
    } else {
        null
    }
}

func main() -> lang.i64 {
     println(findValue(1) ?? 0);
     println(findValue(2) ?? 0);
    0
}
