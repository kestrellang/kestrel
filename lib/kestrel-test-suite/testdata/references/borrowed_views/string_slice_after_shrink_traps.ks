// test: execution
// stdlib: true
// expect-exit: -1

// A slice whose string got shorter than the slice panics on the next
// read instead of reading past the string's bytes.

module Test

import std.text.(String)

@main
func main() -> Int64 {
    var s: String = "hello world";
    let tail = s.asSlice().subslice(from: 6, to: 11);
    s.clear();
    let owned = tail.toOwned();
    if owned.bytes.count == 5 { return 0 }
    1
}
