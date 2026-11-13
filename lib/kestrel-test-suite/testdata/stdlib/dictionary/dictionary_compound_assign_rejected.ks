// test: diagnostics
// stdlib: true

// Plain `=` on `d(key)` goes through its setter, but compound assignment
// needs the current value, which may be missing: still E214. The fix is the
// `default:` subscript or a `&mutating` binder.

module Test

@main
func main() {
    var d = ["a": 1];
    d("a") += 1; // ERROR: cannot assign to failable place 'subscript'
    d("a", default: 0) += 1;
    if let .Some(&mutating v) = d("a") { v += 1; }
}
