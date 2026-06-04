// test: execution
// stdlib: true

// Regression: a closure that reads a Copyable field (`self.cap`) of a
// non-Copyable receiver must capture the *place* `self.cap` (an Int64), not
// the whole `self`. The `and` operator desugars its RHS (`index < self.cap`)
// into a closure; before place-based capture this duplicated the non-Copyable
// `self` and failed copy-check.

module Main

struct Box: not Copyable {
    let cap: std.numeric.Int64

    func clamped(at index: std.numeric.Int64) -> std.numeric.Int64 {
        if index >= 0 and index < self.cap {
            index
        } else {
            -1
        }
    }
}

func main() -> lang.i64 {
    let b = Box(cap: 4);
    let inside = b.clamped(at: 2);
    if inside == 2 { 0 } else { 1 }
}
