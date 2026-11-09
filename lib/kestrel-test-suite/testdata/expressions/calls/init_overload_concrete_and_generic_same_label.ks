// test: execution
// stdlib: true
// expect-exit: 0

module Main

import std.core.(Convertible)

// Two inits share the label `from`: a concrete one that witnesses
// `Convertible[Tag]`, and one generic in its own parameter. The argument's
// type picks between them. The generic one used to be folded into the
// protocol requirement, pinning its `I` to `Tag` and rejecting `[1, 2]`.

struct Tag { let n: Int64 }

struct S { let n: Int64 }

extend S: Convertible[Tag] {
    init(from value: Tag) { self.n = value.n; }
}

extend S {
    init[I](from xs: I) where I: Iterable, I.Item = Int64 { self.n = 99; }
}

@main
func main() -> Int64 {
    if S(from: Tag(n: 1)).n != 1 { return 1 }
    if S(from: [1, 2]).n != 99 { return 2 }
    // The stdlib pair that found it: `String(from: CString)` and
    // `String[I](from chars: I)`.
    if String(from: "hey".chars) != "hey" { return 3 }
    let c = "c str".toCString();
    let back = String(from: c);
    c.free();
    if back != "c str" { return 4 }
    0
}
