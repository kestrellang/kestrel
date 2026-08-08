// test: execution
// stdlib: true
// expect-exit: 0

// E212 IS RETIRED (docs/design/closures.md, Diagnostics table: "E212 — retired
// — view capture is now the default"; plan lockstep 6). The previous version
// of this file asserted E212 here and its own comment anticipated the change:
// "In 2a this keeps every closure Static; 2c relaxes it to 'capture makes the
// closure non-Static'".
//
// That is where we landed. A VIEW-kind closure's environment is a set of
// addresses into the frame that built it, so capturing a `not Static` value is
// sound — the closure cannot outlive the frame (E494). Capturing simply makes
// the closure itself non-Static, and passing it to a parameter that only calls
// it stays legal.
//
// The OWNING tier is the opposite case and keeps its rejection (E624): an
// owned environment snapshots its captures and may outlive the frame.

module Test

import std.numeric.(Int64)

struct Handle: not Static {
    var id: Int64
}

func consume(f: () -> Int64) -> Int64 {
    f()
}

func good(h: Handle) -> Int64 {
    consume { h.id }
}

@main
func main() -> Int64 {
    let h = Handle(id: 7);
    if good(h) != 7 { return 1 }
    0
}
