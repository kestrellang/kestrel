// test: execution
// stdlib: true

// KNOWN BUG (pre-existing, not yet fixed) — characterization test.
//
// Appending a *heap-backed* `String` (built/derived at runtime, with a real
// refcounted `RcBox`) to an `Array[String]` that is a STRUCT FIELD corrupts the
// stored string: reading it back crashes (the buffer is freed while the array
// still references it). The sibling test `cross_module_field_subscript.ks`
// passes because it appends a *string literal* — literals live in static storage
// whose RcBox is immortal, so the missing retain/over-release is invisible.
// A LOCAL `Array[String]` (not a struct field) also works with heap strings.
//
// Distinguishing factor — only the combination breaks:
//   heap String  ×  struct-field Array  ×  read-back
// Local array + heap string: OK.  Struct field + literal: OK.
//
// This blocks talon-sqlite / perch / kestrel-wall: `Headers.parse` stores
// `subslice(...).trimmed().toOwned()` (heap) strings into `self.entries`
// (`Array[(String,String)]`), and `Headers.value(forName:)` then reads them →
// heap corruption while parsing the first HTTP request.
//
// WHEN FIXED this test passes; until then it fails (crash / wrong value), which
// is the signal that the field-array-of-heap-elements refcount path is repaired.
module Test

struct Bag {
    var items: std.collections.Array[std.text.String]
    init() { self.items = std.collections.Array[std.text.String](); }
}

// Build a heap-backed String (not a literal).
func heapString() -> std.text.String {
    var s = std.text.String();
    s.append("Hello");
    s.append(" World");
    s
}

@main
func main() -> lang.i64 {
    var bag = Bag();
    bag.items.append(heapString());
    if bag.items.count != 1 { return 1; };
    let got = bag.items(0);
    if got.isEqual(to: "Hello World") { 0 } else { 2 }
}
