// test: execution
// stdlib: true
// expect-exit: 0

// Pins the kind prefix in a *struct field* type. An `escaping` field keeps the
// struct storable long-term and makes it Cloneable through the aggregate fold,
// so copying the struct shares (never bit-copies) the environment handle —
// both handles drive one counter.
// See docs/design/closures.md — "escaping: a shared, stateful object".
module Test

import std.numeric.(Int64)

func makeCounter(start: Int64) -> escaping () -> Int64 {
    var count = start;
    { count = count + 1; count }
}

struct Button {
    let onClick: escaping () -> Int64
}

@main
func main() -> lang.i64 {
    let b = Button(onClick: makeCounter(0));
    if (b.onClick)() != 1 { return 1 }

    let alias = b;
    if (alias.onClick)() != 2 { return 2 }
    if (b.onClick)() != 3 { return 3 }

    0
}
