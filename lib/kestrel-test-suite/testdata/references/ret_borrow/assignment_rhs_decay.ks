// test: execution
// stdlib: true
// backends: cranelift,llvm

// Regression (#194): a ref-returning call on a plain assignment RHS must decay
// to the pointee, just like binding/return position. Before the fix
// `bind_call_result` bound the call result to the raw `&Int64` before the
// assignment coerce could decay it, manufacturing "expected Int64 got &Int64".
// Also covers a ref RHS written through a ref place (`m.poke() = b.peek()`).
module Test

struct Box {
    var v: Int64
    func peek() -> &Int64 { self.v }
    mutating func poke() -> &mutating Int64 { self.v }
}

@main
func main() -> lang.i64 {
    let b = Box(v: 7);
    var x: Int64 = 0;
    x = b.peek();        // plain assignment RHS decays
    if x != 7 { return 1; }

    var m = Box(v: 1);
    m.poke() = b.peek(); // ref RHS decays, stored through the ref place
    if m.v != 7 { return 2; }
    0
}
