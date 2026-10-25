// test: execution
// stdlib: false
// backends: cranelift,llvm
//
// `@align(N)` raises a type's alignment to at least N and pads its size to a
// multiple of it, so arrays of the type stay aligned. It never lowers the
// natural alignment. A one-field struct whose `@align` pads it is no longer
// carried as its field's scalar, and its value must survive a round trip.

module Main

@align(16)
struct Vec4 { var x: lang.i32; var y: lang.i32; var z: lang.i32; var w: lang.i32 }

@align(16)
struct Wide { var v: lang.i32 }

// Below the natural alignment: no effect.
@align(2)
struct Natural { var v: lang.i64 }

@layout(.C)
@align(8)
struct CAligned { var a: lang.i8; var b: lang.i8 }

@align(8)
enum Flag {
    case Off
    case On
}

struct Holder { var tag: lang.i8; var wide: Wide }

func roundTrip(w: Wide) -> Wide { w }

func isOn(f: Flag) -> lang.i64 {
    match f {
        .On => 1,
        .Off => 0,
    }
}

@main
func main() -> lang.i64 {
    if lang.i64_ne(lang.sizeof[Vec4](), 16) { return 1; }
    if lang.i64_ne(lang.alignof[Vec4](), 16) { return 2; }
    if lang.i64_ne(lang.sizeof[Wide](), 16) { return 3; }
    if lang.i64_ne(lang.alignof[Wide](), 16) { return 4; }
    if lang.i64_ne(lang.alignof[Natural](), 8) { return 5; }
    if lang.i64_ne(lang.sizeof[CAligned](), 8) { return 6; }
    if lang.i64_ne(lang.alignof[CAligned](), 8) { return 7; }
    if lang.i64_ne(lang.sizeof[Flag](), 8) { return 8; }
    // `wide` sits at offset 16, after the one-byte tag.
    if lang.i64_ne(lang.sizeof[Holder](), 32) { return 9; }

    let w = roundTrip(Wide(v: 42));
    if lang.i32_ne(w.v, 42) { return 10; }
    let h = Holder(tag: 1, wide: Wide(v: 7));
    if lang.i32_ne(h.wide.v, 7) { return 11; }
    if lang.i64_ne(isOn(.On), 1) { return 12; }
    if lang.i64_ne(isOn(.Off), 0) { return 13; }
    0
}
