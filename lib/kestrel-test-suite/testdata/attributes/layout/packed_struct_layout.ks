// test: execution
// stdlib: false
// backends: cranelift,llvm
//
// `@layout(.Packed)` places every field at alignment 1 with no padding, so
// `length` sits at offset 1. Reads copy the field out, whole-field writes
// store it, and a field passed to a plain (borrowing) parameter is a copy.
// A struct nested in a packed one is just as misaligned.

module Main

@layout(.Packed)
struct WireHeader { var tag: lang.i8; var length: lang.i32 }

struct Inner { var a: lang.i32; var b: lang.i64 }

@layout(.Packed)
struct Frame { var kind: lang.i8; var inner: Inner; var checksum: lang.i16 }

struct Pair { var first: WireHeader; var second: WireHeader }

func read(n: lang.i32) -> lang.i32 { n }

@main
func main() -> lang.i64 {
    if lang.i64_ne(lang.sizeof[WireHeader](), 5) { return 1; }
    if lang.i64_ne(lang.alignof[WireHeader](), 1) { return 2; }
    // Inner is 16 bytes (natural layout inside), placed at offset 1.
    if lang.i64_ne(lang.sizeof[Frame](), 19) { return 3; }
    if lang.i64_ne(lang.sizeof[Pair](), 10) { return 4; }

    var h = WireHeader(tag: 3, length: 1000);
    if lang.i32_ne(read(h.length), 1000) { return 5; }
    h.length = 2000;
    if lang.i32_ne(h.length, 2000) { return 6; }
    if lang.i8_ne(h.tag, 3) { return 7; }

    var f = Frame(kind: 1, inner: Inner(a: 5, b: 6), checksum: 9);
    f.inner.b = 60;
    if lang.i64_ne(f.inner.b, 60) { return 8; }
    if lang.i32_ne(f.inner.a, 5) { return 9; }
    if lang.i16_ne(f.checksum, 9) { return 10; }

    let p = Pair(first: WireHeader(tag: 1, length: 11), second: WireHeader(tag: 2, length: 22));
    if lang.i32_ne(p.second.length, 22) { return 11; }
    0
}
