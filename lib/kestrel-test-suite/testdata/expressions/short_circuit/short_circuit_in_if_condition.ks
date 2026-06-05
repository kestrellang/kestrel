// test: diagnostics

module Main
import std.io.stdio.println

func effect() -> Bool {
     println("EFFECT");
    true
}

func main() -> lang.i64 {
    // false and should not call effect
    if false and effect() {
         println("yes");
    } else {
         println("no");
    }

    // true or should not call effect
    if true or effect() {
         println("yes");
    } else {
         println("no");
    }
    0
}
