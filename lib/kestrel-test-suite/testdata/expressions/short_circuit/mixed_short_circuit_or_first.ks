// test: diagnostics

module Main
import std.io.stdio.println

func effect() -> Bool {
     println("EFFECT");
    true
}

func main() -> lang.i64 {
    let result = true or effect() and effect();
     println(result);
    0
}
