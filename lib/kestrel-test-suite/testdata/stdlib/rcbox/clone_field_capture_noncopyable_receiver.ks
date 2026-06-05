// test: execution
// stdlib: true

// A closure that reads a Cloneable (RcBox) field of a *non-Copyable* receiver
// must place-capture the field — clone just `c.box` into the env and borrow
// `c` — never capturing the whole receiver. Capturing the whole non-Copyable
// `c` would move/consume it, so the use of `c` after the closure would fail to
// compile (use of moved value). This passing at all proves `c` survives the
// capture; the value checks prove the captured field is the real one.

module Test

struct Container: not Copyable {
    var box: std.memory.RcBox[std.numeric.Int64]
}

@main
func main() -> lang.i64 {
    let c = Container(box: std.memory.RcBox[std.numeric.Int64](42));

    let getter = { c.box.getValue() };
    if getter() != 42 { return 1 }           // closure captured the field's value
    if c.box.getValue() != 42 { return 2 }   // `c` still usable — not consumed

    0
}
