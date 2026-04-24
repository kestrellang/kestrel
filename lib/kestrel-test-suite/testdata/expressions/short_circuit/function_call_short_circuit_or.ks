// test: diagnostics

module Main
import std.io.stdio.println

func alwaysFalse() -> Bool {
    let _ = println("FALSE");
    false
}

func alwaysTrue() -> Bool {
    let _ = println("TRUE");
    true
}

func main() -> lang.i64 {
    let r = alwaysTrue() or alwaysFalse();
    let _ = println(r);
    0
}
