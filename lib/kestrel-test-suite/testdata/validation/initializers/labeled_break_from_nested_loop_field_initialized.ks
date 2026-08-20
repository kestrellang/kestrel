// test: diagnostics
// stdlib: false
//
// The other side of G10: the same shape, but the field IS assigned on the path
// that reaches the labeled break, so the merged break state has it and E005
// must NOT fire. Keying frames by label must not turn a valid initializer into
// a false reject.

module Main

struct S {
    var a: lang.i64

    init() {
        outer: loop {
            loop {
                self.a = 7;
                break outer;
            }
        }
    }
}
