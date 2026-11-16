// test: diagnostics
// stdlib: true
// include: file_scoped_helpers.ks

// Top-level `private` and `fileprivate` declarations are file-scoped, so a
// second file of the same module can't use them. Members already enforce
// this (E115); top-level functions and types don't yet.
module Test

func useThem() -> Int64 {
    let a = privateHelper(); // ERROR: 'privateHelper' is private and not accessible from this scope
    let b = fileprivateHelper(); // ERROR: 'fileprivateHelper' is fileprivate and not accessible from this scope
    let c = FileprivateBox(v: 3); // ERROR: 'FileprivateBox' is fileprivate and not accessible from this scope
    a + b + c.v
}
