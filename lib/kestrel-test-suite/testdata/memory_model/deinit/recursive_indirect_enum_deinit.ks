// test: diagnostics
// stdlib: true

module Test

import std.numeric.Int64

public var deinit_count: Int64 = 0;

struct Payload: not Copyable {
    var id: Int64
    deinit {
        deinit_count = deinit_count + 1;
    }
}

indirect enum Tree: not Copyable { // ERROR: indirect enums are not yet supported
    case Leaf(value: Payload)
    case Node(left: Tree, right: Tree)
}
