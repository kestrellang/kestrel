// test: diagnostics

module Main
import std.io.stdio.println

func main() -> lang.i64 {
    if true or false {
        let _ = println("at least one");
    } else {
        let _ = println("neither");
    }

    if false or false {
        let _ = println("at least one");
    } else {
        let _ = println("neither");
    }
    0
}
