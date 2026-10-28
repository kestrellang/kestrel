// test: diagnostics

module Main
import std.io.stdio.println

func main() -> lang.i64 {
    if true or false {
         println("at least one");
    } else {
         println("neither");
    }

    if false or false {
         println("at least one");
    } else {
         println("neither");
    }
    0
}
