// test: diagnostics
// stdlib: false

module Test
private struct Hidden { }
public type Exposed = Hidden; // ERROR: aliased type in 'Exposed' is less visible
