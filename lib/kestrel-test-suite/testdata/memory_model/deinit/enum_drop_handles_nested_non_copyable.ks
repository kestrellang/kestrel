// test: diagnostics
// stdlib: false

module Test
import Prelude

struct Inner: not Copyable {
    var id: lang.i64
    deinit {}
}

struct Outer: not Copyable {
    var inner: Inner
    deinit {}
}

enum Container: not Copyable {
    case wrapped(value: Outer)
    case empty
}

func example() {
    let c = Container.wrapped(value: Outer(inner: Inner(id: 1)));
}
