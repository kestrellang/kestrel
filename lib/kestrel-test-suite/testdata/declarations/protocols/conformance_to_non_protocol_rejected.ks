// test: diagnostics

// A conformance list entry that names a non-protocol type is rejected (it
// used to be dropped silently).

module Test

protocol P { }

struct S: P { }

struct T: S { } // ERROR: 'S' is a struct, not a protocol

struct U: U { } // ERROR: 'U' is a struct, not a protocol

