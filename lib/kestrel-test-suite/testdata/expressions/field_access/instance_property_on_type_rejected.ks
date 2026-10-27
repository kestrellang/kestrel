// test: diagnostics

// An instance property needs an instance; naming it through the type has
// nothing to read from (it used to lower as a global and fail in codegen).
// Static properties are read through the type.

module Test

struct S {
    var v: Int64;
    static let w: Int64 = 3;
}

func read() -> Int64 {
    let a = S.w;
    let b = S.v; // ERROR: instance property 'v' cannot be accessed on a type
    a + b
}
