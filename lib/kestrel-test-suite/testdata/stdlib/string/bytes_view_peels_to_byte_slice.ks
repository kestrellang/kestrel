// test: execution
// stdlib: true
// expect-stdout: 2 3 2 2 3 1 true\n

// `String.bytes` is a `BytesView` that peels to `Slice[UInt8]` through
// `Indirection`, so it passes to a `&Slice[UInt8]` param in every argument
// shape: inline on an unannotated literal binding (#138), on a `String`
// param, bound first, as a sub-view, and into a protocol requirement.
module Test

import std.collections.(Slice, DefaultHasher)

func count(bytes: &Slice[UInt8]) -> Int64 { bytes.count }

func viaParam(text: String) -> Int64 { count(text.bytes) }

@main
func main() -> lang.i64 {
    let s = "ab";
    let t: String = "abc";
    let bound = s.bytes;
    let slice = t.asSlice();
    var a = DefaultHasher();
    a.write("hi".bytes);
    var b = DefaultHasher();
    b.write("h".bytes);
    b.write("i".bytes);
    print("\(count(s.bytes)) \(count(t.bytes)) \(count(bound)) \(viaParam(s)) \(count(slice.bytes)) \(count(t.bytes(1..<2))) \(a.finish() == b.finish())\n");
    0
}
