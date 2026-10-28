// test: execution
// stdlib: true
// expect-exit: 0
//
// On an existential, the protocol's members win over the box's own: a
// requirement named `isUnique` calls the payload, not `RcBox.isUnique()`
// (design §7).

module Test

import std.numeric.(Int64)
import std.core.(Bool)

protocol Probe {
    func isUnique() -> Bool
}

struct Always {}
extend Always: Probe {
    public func isUnique() -> Bool { false }
}

@main
func main() -> Int64 {
    let p: any Probe = Always();
    // RcBox.isUnique() would be true here: one handle.
    if p.isUnique() { return 1 }
    0
}
