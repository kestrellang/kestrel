// test: execution
// stdlib: true
// expect-exit: 0

module Test

import std.numeric.(Int64)

struct Res: not Copyable {
    var id: Int64
    func peek() -> Int64 { self.id }   // borrows self
    deinit { }
}

// #177, re-baselined for the VIEW tier (docs/design/closures.md, "Behavior
// Changes from Today" #3: "Capturing a non-Copyable value no longer kills the
// original in view kinds — it is merely frozen against destruction").
//
// A normal closure's environment holds an ADDRESS of `r`, so capturing moves
// nothing: `r` stays live and usable afterwards. The front-end records no
// capture move and MIR emits no `Take` — the two halves must stay in lockstep,
// or this shape is back to the OSSA "consumed more than once" ICE that #177
// was filed for. What is still rejected is DESTROYING `r` while the view is
// live (the freeze rule, E507) and moving it OUT of the closure body (E506).
@main
func main() -> lang.i64 {
    let r = Res(id: 7);
    let f = { () in r.peek() };   // view of `r` — no move
    if f() != 7 { return 1 }
    if r.peek() != 7 { return 2 }  // `r` still live: no use-after-move
    if f() != 7 { return 3 }
    0
}
