// test: execution
// stdlib: true
// expect-exit: 0

module Test

import std.text.(Formatter, StringBuilder)

// A `not Static` param may hold a reference, so a ref argument binds it to
// the ref (as Rust binds `T = &mut S`), not to a copy of the pointee: the
// writes reach `b`. Fails today: `W` infers as `StringBuilder` and the
// holder writes to a clone.
extend &mutating F: Formatter where F: Formatter {
    public mutating func append(other: &StringSlice) { self.append(other) }
    public mutating func append(char c: Char) { self.append(char: c) }
}

struct Holder[W] where W: Formatter, W: not Static {
    var w: W
    mutating func hi() { self.w.append("H") }
}

@main
func main() -> Int64 {
    var b = StringBuilder();
    let r = &mutating b;
    var h = Holder(w: r);
    h.hi();
    b.append("!");
    if b.build() != "H!" { return 1 }
    0
}
