// test: diagnostics

module Main
import std.io.stdio.println

func effect() -> Bool {
     println("EFFECT");
    true
}

func main() -> lang.i64 {
    let result = not false or effect();
     println(result);
    0
}
