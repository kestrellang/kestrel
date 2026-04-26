// test: diagnostics
// stdlib: false

module Test
@builtin(.Copyable)
protocol Copyable { // ERROR: must be a marker protocol
    type Element;
}
