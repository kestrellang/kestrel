// test: diagnostics

module Main
import std.io.stdio.println

func main() -> lang.i64 {
    if true and true {
         println("both true");
    } else {
         println("not both");
    }

    if true and false {
         println("both true");
    } else {
         println("not both");
    }
    0
}
