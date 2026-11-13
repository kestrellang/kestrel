// test: execution
// stdlib: true
// expect-stdout: abab\nxy  xy\n

// Appending a slice of the string itself: growing the buffer moves the bytes
// the slice points at, so `append` re-finds them in the new buffer.

module Test

import std.text.(String)

@main
func main() {
    var s: String = "ab";
    s.append(s);
    println(s);
    var t: String = "xy  ";
    t.append(t.trimmed());
    println(t);
}
