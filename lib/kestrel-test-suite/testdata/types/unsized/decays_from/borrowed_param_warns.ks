// test: diagnostics
// stdlib: true

// E512: a borrowed `String` parameter given a slice converts it, but the
// callee only reads, so taking `&StringSlice` would avoid the copy.

module Test

import std.numeric.(Int64)

func see(x: String) -> Int64 { x.bytes.count }

@main
func main() {
    let s = "  hi  ";
    let n = see(s.trimmed()); // WARNING: passing a 'StringSlice' to a borrowed 'String' parameter copies it
}
