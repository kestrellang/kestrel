// test: mir
// stdlib: false
// mir-filter: Test.example

// Scenario: two non-Copy locals declared in order `a, b`. Drops must
// appear in reverse declaration order — `drop %b` before `drop %a` —
// before the function's `return`.

module Test
import Prelude

@builtin(.Copyable)
protocol Copyable {}

struct Handle: not Copyable {
    var fd: lang.i64
    deinit {}
}

func example() {
    let a = Handle(fd: 1);
    let b = Handle(fd: 2);
}
