// test: diagnostics

module Main
import std.io.stdio.println

struct Flags {
    let enabled: Bool
    let visible: Bool
}

func main() -> lang.i64 {
    let f1 = Flags(enabled: true, visible: true);
    let f2 = Flags(enabled: true, visible: false);
     println(f1.enabled and f1.visible);
     println(f2.enabled and f2.visible);
    0
}
