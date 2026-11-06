// test: diagnostics
// stdlib: true

// Assigning to a range place is rejected, as in Rust: a slice is unsized and
// can't be assigned as a whole. Copy into it with
// `xs(1..<3).assign(from: ys)` (lengths must match), or resize with
// `replaceSubrange`.
module Test

import std.collections.(Array)

func f() {
    var xs = [1, 2, 3, 4];
    let ys = [8, 9];
    xs(1..<3) = ys; // ERROR(E156)
    xs(1..<3).assign(from: ys);
    xs.replaceSubrange(1..<3, with: ys);
}
