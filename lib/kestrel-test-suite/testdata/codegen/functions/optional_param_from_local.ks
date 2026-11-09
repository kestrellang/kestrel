// test: execution
// stdlib: true
// expect-exit: 0

// A non-optional local passed straight to an optional parameter is wrapped in
// `.Some`. The by-borrow argument path used to lend the local's slot as-is, so
// the callee read the unwrapped `String`/`Int64` as `.None`.
module Test

import std.numeric.(Int64)
import std.text.(String)

func show(s: String?) -> Int64 {
    match s { .Some(x) => x.bytes.count, .None => -1 }
}

func showInt(n: Int64?) -> Int64 {
    match n { .Some(x) => x, .None => -1 }
}

func forward(s: String) -> Int64 { show(s) }

struct Box {
    var name: String;
    func measure(s: String?) -> Int64 { show(s) }
}

@main
func main() -> Int64 {
    let v: String = "hello";
    if show(v) != 5 { return 1; }
    if show("lit") != 3 { return 2; }
    var w: String = "abcd";
    if show(w) != 4 { return 3; }
    let k: Int64 = 9;
    if showInt(k) != 9 { return 4; }
    var j: Int64 = 7;
    if showInt(j) != 7 { return 5; }
    if forward("xy") != 2 { return 6; }
    let b = Box(name: "box");
    if b.measure(v) != 5 { return 7; }
    if show(b.name) != 3 { return 8; }
    // The locals are still usable after being lent.
    if v.bytes.count + w.bytes.count != 9 { return 9; }
    0
}
