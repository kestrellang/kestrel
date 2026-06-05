// test: execution
// stdlib: true
// expect-exit: 0

module Test

import std.numeric.Int64

public var deinit_count: Int64 = 0;

struct Resource: not Copyable {
    var id: Int64
    deinit {
        deinit_count = deinit_count + 1;
    }
}

enum Container: not Copyable {
    case Full(value: Resource)
    case Empty
}

func consume(consuming r: Resource) {}

@main
func main() -> lang.i64 {
    let c1 = Container.Full(value: Resource(id: 1));
    match c1 {
        .Full(value: v) => {
            consume(v);
        },
        .Empty => {}
    }
    if deinit_count != 1 { return 1; }

    let c2 = Container.Empty;
    match c2 {
        .Full(value: v) => {
            consume(v);
        },
        .Empty => {}
    }
    // No additional deinit for Empty variant
    if deinit_count != 1 { return 2; }

    0
}
