// test: diagnostics
// stdlib: false

// The sibling of instance_method_on_type_as_value.ks: a method reached through
// an *instance* rather than its type. `b.doubled` lowers to a field access, so
// the rejection comes from inference (E100 MethodNotCalled) rather than HIR
// lowering — different path, same rule, so both need pinning.
// `primitive_methods_errors.ks` covers the primitive-receiver variant.

module Main

struct Box {
    let v: lang.i64
    func doubled(x: lang.i64) -> lang.i64 { 0 }
}

func test(b: Box) -> () {
    let f = b.doubled; // ERROR: method 'doubled' on 'Box' must be called
}
