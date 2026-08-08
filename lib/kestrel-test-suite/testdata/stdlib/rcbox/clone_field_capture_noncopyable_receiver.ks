// test: execution
// stdlib: true

// A closure that reads a Cloneable (RcBox) field of a *non-Copyable* receiver
// must place-capture the field — the env holds a view of just `c.box`, never
// the whole receiver (docs/design/closures.md, "What is captured: the narrowest
// place"). Under the VIEW tier the capture neither clones the field nor
// consumes `c`: the env holds the field's ADDRESS and owns nothing, so `c`
// stays usable afterwards. This passing at all proves `c` survives the capture;
// the value checks prove the viewed field is the real one.

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
