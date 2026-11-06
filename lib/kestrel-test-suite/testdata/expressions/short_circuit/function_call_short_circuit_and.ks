// test: diagnostics

module Main
import std.streams.println

func alwaysFalse() -> Bool {
     println("FALSE");
    false
}

func alwaysTrue() -> Bool {
     println("TRUE");
    true
}

func main() -> lang.i64 {
    let r = alwaysFalse() and alwaysTrue();
     println(r);
    0
}
