// test: execution
// stdlib: true
// expect-exit: 0

// SCOPE NOTE — this is a REGRESSION BACKSTOP, not a leak assertion.
//
// Failable-init half of `init_field_reassign_string_no_crash.ks`. The
// fragility-audit G1 leak was measured on exactly this shape with the darwin
// `leaks` tool — a failable `init?` that assigns a heap-owning `String` field
// and then `return null`s. This exact file, compiled by the pre-fix compiler,
// reports `200 leaks for 132800 total leaked bytes` (one per failing
// iteration); post-fix it reports 0. But `leaks` is darwin-only and not
// suite-runnable, and this program EXITS 0 either way.
// Nothing portable can observe a freed `String` buffer from inside a Kestrel
// program (`String`'s allocator has no user-visible counter hook), so this file
// does NOT claim to detect the leak; the `deinit`-counter fixtures beside it do
// (`partial_drop_on_init_failure_transitive_cloneable.ks`,
// `partial_drop_on_init_failure_default_copyable_field_drop.ks`).
//
// What it IS worth: the fix arms the failure-return guarded-destroy diamond for
// a real stdlib heap type. This loops the shape 200×, asserting the failing
// inits return null and the succeeding ones hand back an intact string — which
// is what would catch a double-free of the abandoned buffer. A regression shows
// up as a crash or a malloc abort, not as a diagnostic.

module Test

import std.numeric.Int64
import std.text.String

struct Holder: not Copyable {
    var s: String
    var v: Int64
    init(v v: Int64)? {
        self.s = makeBig(v);            // live when the failure return fires
        if v < 100 { return null }      // abandons a partially built `self`
        self.v = v;
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
    var built: Int64 = 0;
    var failed: Int64 = 0;
    for i in 0..<200 {
        let h = Holder(v: i);
        match h {
            .Some(c) => {
                if c.s == makeBig(i) { built = built + 1; }
            },
            _ => { failed = failed + 1; }
        }
    }
    if failed != 100 { return 1 }
    if built != 100 { return 2 }
    0
}
