// test: execution
// stdlib: true
// expect-exit: 0

// docs/design/shared-box.md: "dropping the last handle destroys the payload
// exactly once" — release is the conformer's own `deinit` under ordinary drop
// rules. A deinit-counting payload pins both halves: no destruction while any
// handle is alive, exactly one destruction at the last release (never two, one
// per handle).
module Test

import std.memory.(RcBox)
import std.numeric.(Int64)

public var deinit_count: Int64 = 0;

struct Payload: not Copyable {
    var id: Int64
    deinit { deinit_count = deinit_count + 1; }
}

// Two handles onto one payload; both are released when this returns.
func shareThenRelease() -> Int64 {
    let box = RcBox(Payload(id: 7));
    let alias = box.clone();
    let id: Int64 = alias.pointeeRef().id;
    if deinit_count != 0 { return -1; }   // nothing destroyed while handles are live
    id
}

@main
func main() -> lang.i64 {
    if shareThenRelease() != 7 { return 1; }
    if deinit_count != 1 { return 2; }    // exactly once, at the last release
    0
}
