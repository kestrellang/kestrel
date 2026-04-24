// test: diagnostics

module Main
import std.io.stdio.println

func main() -> lang.i64 {
    if true and true {
        let _ = println("both true");
    } else {
        let _ = println("not both");
    }

    if true and false {
        let _ = println("both true");
    } else {
        let _ = println("not both");
    }
    0
}
