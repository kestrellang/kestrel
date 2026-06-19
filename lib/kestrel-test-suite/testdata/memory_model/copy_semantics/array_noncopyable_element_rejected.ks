// test: diagnostics
// stdlib: true

// Regression (#142): instantiating a generic that requires copyable/cloneable
// elements with a `not Copyable` element must be rejected at instantiation —
// not silently miscompiled (the old behavior corrupted memory in
// `Array.retain` and ICE'd on unconstrained generic structs). `Array[Named]`
// drives the `ArrayStorage[T]: Cloneable` conformance, which `Named` cannot
// satisfy.

module Test

struct Named: not Copyable {
    var name: String
}

func test() {
    var arr = Array[Named](); // ERROR: does not satisfy constraint: Named !: Copyable
}
