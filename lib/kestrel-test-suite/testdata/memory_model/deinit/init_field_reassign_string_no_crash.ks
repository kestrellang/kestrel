// test: execution
// stdlib: true
// expect-exit: 0

// SCOPE NOTE — this is a REGRESSION BACKSTOP, not a leak assertion.
//
// The fragility-audit G1 leak was *measured* on exactly this shape with the
// darwin `leaks` tool: a `Holder.init` assigning a heap-owning `String` field
// twice, 200 iterations. THIS FILE, compiled by the pre-fix compiler, reports
// `400 leaks for 265600 total leaked bytes`; post-fix it reports 0. But `leaks`
// is darwin-only and not suite-runnable, and this program EXITS 0 either way.
// There is NO portable, in-suite way to assert that
// the abandoned buffer was freed — `String`'s allocator exposes no user-visible
// counter hook, and only an external tool can prove it. So this file does NOT
// claim to detect the leak. The `deinit`-counter fixtures next to it do that
// (`init_field_reassign_transitive_cloneable.ks`,
// `init_field_reassign_default_copyable_field_drop.ks`).
//
// What it IS worth: G1's fix arms the drop path for a real stdlib `Cloneable`
// heap type in an init body for the first time. This loops the shape 200× and
// asserts the surviving content is exactly the SECOND assignment — proving the
// reassignment lowered to `store_assign` (which carries the destroy-old
// expansion) and did not double-free or corrupt the live buffer. A regression
// here shows up as a crash or wrong content, not as a diagnostic.

module Test

import std.numeric.Int64
import std.text.String

struct Holder: not Copyable {
    var s: String
    init(n n: Int64) {
        self.s = makeBig(n);        // allocates a buffer
        self.s = makeBig(n + 1);    // the first buffer must be released here
    }
}

func makeBig(n: Int64) -> String {
    var b = "";
    for i in 0..<64 {
        b = b + "0123456789abcdef";
    }
    b + "\(n)"
}

@main
func main() -> lang.i64 {
    var ok: Int64 = 0;
    for i in 0..<200 {
        let h = Holder(n: i);
        // The surviving value must be the SECOND assignment, intact.
        if h.s == makeBig(i + 1) { ok = ok + 1; }
    }
    if ok != 200 { return 1 }
    0
}
