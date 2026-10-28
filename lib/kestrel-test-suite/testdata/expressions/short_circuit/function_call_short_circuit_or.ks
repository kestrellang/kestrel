// test: diagnostics

module Main
import std.io.stdio.println

func alwaysFalse() -> Bool {
     println("FALSE");
    false
}

func alwaysTrue() -> Bool {
     println("TRUE");
    true
}

func main() -> lang.i64 {
    let r = alwaysTrue() or alwaysFalse();
     println(r);
    0
}
