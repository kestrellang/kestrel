// test: diagnostics
// stdlib: false
//
// E643: a conformance to an `unsafe protocol` must be written `unsafe P`,
// since the conformer vouches for the protocol's contract.
// E644: `unsafe` on a conformance to a safe protocol has no effect.

module Test

unsafe protocol Trusted {
    func len() -> lang.i64
}

protocol Plain {
    func len() -> lang.i64
}

struct A: Trusted { // ERROR: 'Trusted' is an unsafe protocol; write the conformance as `unsafe Trusted`
    func len() -> lang.i64 { 0 }
}

struct B: unsafe Trusted {
    func len() -> lang.i64 { 0 }
}

struct C {}

extend C: unsafe Plain { // WARN: 'Plain' is not an unsafe protocol; `unsafe` here has no effect
    func len() -> lang.i64 { 0 }
}
