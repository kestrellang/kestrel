// test: execution
// stdlib: true
// expect-exit: 0

// Guard for the F10 fix. Static-member lookup now sources candidates from
// `TypeMembersByName` (which merges type extensions AND conforming-protocol
// extensions) instead of stopping at the first matching extension. The merge
// must not flatten precedence: a protocol-extension default joins the overload
// set only when its LABEL SIGNATURE is not already taken by a direct or
// own-extension candidate. So `A`'s own `tag()` wins over the `Tagged` default
// without the two becoming ambiguous, while `B` — which contributes nothing —
// still reaches the default. Same rule as instance members in
// `kestrel_type_infer::resolve_member`.

module Test

import std.numeric.Int64

protocol Tagged {}

extend Tagged { static func tag() -> Int64 { 1 } }

struct A: Tagged {}
struct B: Tagged {}

extend A { static func tag() -> Int64 { 2 } }

@main
func main() -> lang.i32 {
    if A.tag() != 2 { return 10 }  // type's own static wins
    if B.tag() != 1 { return 20 }  // protocol-extension default still reachable
    0
}
