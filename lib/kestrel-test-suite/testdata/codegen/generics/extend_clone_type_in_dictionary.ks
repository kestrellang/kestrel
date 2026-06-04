// test: execution
// stdlib: true

// Regression: a Cloneable type whose `clone()` is defined in an `extend` block
// (rather than inline in the type body) was not recognized as Clone-behavior,
// so storing it as a generic-container value corrupted it.
//
// Root: clone-shim copy-behavior, mono collection, and the expand pass's
// CopyValue→clone lookup all associated a `.clone` method with its type via the
// `FunctionKind::Method` `parent`. An `extend`-defined `clone()` does not
// reliably set `parent` to the extended type, so the type stayed
// `CopyBehavior::None`, its clone was never collected, and the expand pass
// *degraded* a `CopyValue` on it to a bitwise alias / move. Stored as a
// `Dictionary[String, Boxed]` value, the bucket clone shim then bit-copied the
// `Boxed` (aliasing its inner heap String); reading it back use-after-freed.
// Fix: associate a user `clone()` with its nominal via the **self-param type**
// (`FunctionDef::clone_method_self_nominal`), robust for `extend` and inline.
//
// This was the kestrel-wall HTTP-handler crash: quill's `Value` (an
// `extend Value { func clone() }` enum) used as a `Dictionary[String, Value]`
// JSON value. An inline `clone()` or an auto-synthesized shim was unaffected.
module Test

import std.text.(String)
import std.collections.(Dictionary, Array)

enum Boxed: Cloneable {
    case S(String)
    case Obj(Dictionary[String, Boxed])
    case Arr(Array[Boxed])
    case N
}

// `clone()` defined via `extend` (the trigger), not inline.
extend Boxed {
    public func clone() -> Boxed {
        match self {
            .S(s) => .S(s.clone()),
            .Obj(obj) => .Obj(obj.clone()),
            .Arr(arr) => .Arr(arr.clone()),
            .N => .N
        }
    }
}

@main
func main() -> lang.i64 {
    var d = Dictionary[String, Boxed]();
    d.insert("key", Boxed.S("a-heap-string-value"));
    if d.count != 1 { return 1; };
    // Read back: the stored Boxed.S's String must survive intact.
    match d("key") {
        some b => {
            match b {
                .S(s) => { if not s.isEqual(to: "a-heap-string-value") { return 2; } },
                _ => return 3
            }
        },
        null => return 4
    };
    0
}
