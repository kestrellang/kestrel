// test: diagnostics

module Main
import std.io.stdio.println

struct State {
    let loading: Bool
    let error: Bool
}

func main() -> lang.i64 {
    let s1 = State(loading: false, error: false);
    let s2 = State(loading: true, error: false);
     println(s1.loading or s1.error);
     println(s2.loading or s2.error);
    0
}
