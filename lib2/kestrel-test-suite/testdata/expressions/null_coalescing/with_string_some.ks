// test: diagnostics

module Main
import std.io.stdio.println

func main() -> lang.i64 {
    let name: String? = .Some("Alice");
    let _ = println(name ?? "anonymous");
    0
}
