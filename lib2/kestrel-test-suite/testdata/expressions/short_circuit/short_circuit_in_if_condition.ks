// test: diagnostics

module Main
import std.io.stdio.println

func effect() -> Bool {
    let _ = println("EFFECT");
    true
}

func main() -> lang.i64 {
    // false and should not call effect
    if false and effect() {
        let _ = println("yes");
    } else {
        let _ = println("no");
    }

    // true or should not call effect
    if true or effect() {
        let _ = println("yes");
    } else {
        let _ = println("no");
    }
    0
}
