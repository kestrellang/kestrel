// test: execution
// stdlib: true
// expect-exit: 0

module Test

import std.text.(Formatter, StringBuilder)

// `extend &mutating F: Formatter` forwards to the pointee. Its `mutating`
// self is the ref's slot: the forward must load the ref and write to the
// pointee, not treat the slot as an `F`. A generic param instantiated at
// the ref must receive the ref itself, not a copy of the pointee.
extend &mutating F: Formatter where F: Formatter {
    public mutating func append(other: &StringSlice) { self.append(other) }
    public mutating func append(char c: Char) { self.append(char: c) }
}

func viaGeneric[T](consuming x: T) where T: Formatter, T: not Static, T: not Copyable {
    var y = x;
    y.append("G")
}

struct Holder[W] where W: Formatter, W: not Static {
    var w: W
    mutating func hi() { self.w.append("H") }
}

@main
func main() -> Int64 {
    var b = StringBuilder();
    let r = &mutating b;
    viaGeneric[&mutating StringBuilder](r);
    var h = Holder[&mutating StringBuilder](w: r);
    h.hi();
    b.append("!");
    if b.build() != "GH!" { return 1 }
    0
}
