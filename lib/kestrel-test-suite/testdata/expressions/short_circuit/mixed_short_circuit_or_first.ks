// test: diagnostics

module Main
import std.io.stdio.println

func effect() -> Bool {
    let _ = println("EFFECT");
    true
}

func main() -> lang.i64 {
    let result = true or effect() and effect();
    let _ = println(result);
    0
}
