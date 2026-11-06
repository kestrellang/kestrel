// test: execution
// stdlib: true
// expect-exit: 0

// Text views read through the string on every access. Growing the string
// while a view is alive moves its buffer; the view must read the new
// buffer, not the freed one it was made from.

module Test

import std.text.(String)

@main
func main() -> lang.i64 {
    var s: String = "ab";
    let bytes = s.bytes;
    let chars = s.chars;
    // Force a reallocation.
    for _ in 0..<64 { s.append("xyzw"); }
    if bytes(0) != 97 { return 1 }
    if bytes(1) != 98 { return 2 }
    var it = chars.iter();
    if it.next() != .Some('a') { return 3 }
    if it.next() != .Some('b') { return 4 }
    // A slice made before the change still covers its first bytes.
    let head = s.asSlice().subslice(from: 0, to: 2);
    s.append("!");
    if head.toOwned() != "ab" { return 5 }
    0
}
