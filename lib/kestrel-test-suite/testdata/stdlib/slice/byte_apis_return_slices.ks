// test: execution
// stdlib: true

module Test

import std.collections.(Slice, DefaultHasher, ArrayBuilder)

func countOf(bytes: Slice[UInt8]) -> Int64 { bytes.count }

func sum(xs: Slice[Int64]) -> Int64 {
    var total: Int64 = 0;
    for x in xs { total = total + x }
    total
}

// The public byte and sub-slice APIs hand out safe `Slice` views, not
// `UnsafeSlice`: `String.bytes` (through `Indirection`), `Hasher.write`, `Slice`'s
// prefix/suffix/drop, and `ArrayBuilder.append(contentsOf:)`.
@main
func main() -> lang.i64 {
    // `bytes` binds as a `BytesView` and passes as a `Slice[UInt8]`.
    let bytes = "hey".bytes;
    if bytes.count != 3 { return 1 }
    if bytes(0) != 104 { return 2 }
    if countOf("abcd".bytes) != 4 { return 3 }

    // Hasher.write takes a Slice; split writes equal one write.
    var a = DefaultHasher();
    a.write("hello world".bytes);
    var b = DefaultHasher();
    b.write("hello".bytes);
    b.write(" world".bytes);
    if a.finish() != b.finish() { return 4 }

    // ArrayBuilder.append(contentsOf:) takes a `&Slice`; an array literal peels to one.
    var builder = ArrayBuilder[Int64]();
    builder.append(contentsOf: [1, 2, 3, 4, 5]);
    let arr = builder.build();
    if arr.count != 5 { return 5 }

    // Slice's prefix/suffix/drop (reached here through UnsafeSlice's peel) return Slices.
    let raw = arr.asUnsafeSlice();
    if sum(raw.prefix(2)) != 3 { return 6 }
    if sum(raw.suffix(2)) != 9 { return 7 }
    if sum(raw.drop(first: 3)) != 9 { return 8 }
    if sum(raw.drop(last: 3)) != 3 { return 9 }
    0
}
