// test: diagnostics
// stdlib: true

module Test

import std.numeric.(Int64)

struct Res: not Copyable {
    var id: Int64
    deinit { }
}

func consume(consuming r: Res) -> Int64 { r.id }

// Regression (#177 capture-dup): a closure may be called more than once but
// owns the single non-Copyable value it captured by value. Moving that value
// OUT of the body — returning it (`{ () in r }`) or consuming it
// (`{ () in consume(r) }`) — would duplicate it (double-deinit) and must be
// rejected (E506) rather than silently miscompiled.
func returnsCapture() {
    let r = Res(id: 1);
    let f = { () in r };   // ERROR: cannot move captured value 'r' out of a closure
    let _ = f;
}

func consumesCapture() {
    let s = Res(id: 2);
    let f = { () in consume(s) }; // ERROR: cannot move captured value 's' out of a closure
    let _ = f;
}

func main() -> lang.i64 { 0 }
