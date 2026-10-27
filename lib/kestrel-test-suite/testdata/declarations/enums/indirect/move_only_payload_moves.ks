// test: diagnostics
// stdlib: true
//
// A boxed case with a move-only payload makes the enum move-only: using it
// after a move is an error, and declaring it Copyable is rejected.

module Test

import std.numeric.(Int64)

struct Handle: not Copyable {
    var id: Int64
}

indirect enum Handles: not Copyable {
    case End
    case Link(Handle, Handles)
}

func consume(consuming h: Handles) {}

func twice() {
    let hs = Handles.Link(Handle(id: 1), .End);
    consume(hs);
    consume(hs); // ERROR: use of moved value 'hs'
}

indirect enum Pinned: Copyable { // ERROR: 'Pinned' conforms to `Copyable` but contains non-Copyable field 'Link'
    case End
    case Link(Handle, Pinned)
}
