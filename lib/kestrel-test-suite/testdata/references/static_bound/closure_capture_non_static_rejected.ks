// test: diagnostics
// stdlib: true

// E212 generalized (2a): a closure capturing any non-Static value is
// rejected — the env may outlive the scope. In 2a this keeps every
// closure Static; 2c relaxes it to "capture makes the closure non-Static".

module Test

struct Handle: not Static {
    var id: Int64
}

func consume(f: () -> Int64) -> Int64 {
    f()
}

func bad(h: Handle) -> Int64 {
    consume { h.id } // ERROR(E212)
}
