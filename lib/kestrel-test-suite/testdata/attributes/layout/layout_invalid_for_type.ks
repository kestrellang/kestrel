// test: diagnostics
// stdlib: false
//
// E910: `@layout(.Packed)` is for structs only;
// `@layout(.C)` makes an enum a C `int`, so no case may carry a payload.
// `.Packed` with `@align` is a conflict (E906); `.C` with `@align` is fine.

module Main

@layout(.Packed) // ERROR: '@layout(.Packed)' cannot be applied to an enum
enum PackedEnum {
    case A
    case B
}

@layout(.C) // ERROR: '@layout(.C)' on an enum needs every case to be payload-less; 'Shape.Circle' has a payload
enum Shape {
    case Point
    case Circle(radius: lang.i64)
}

@layout(.C)
enum Plain {
    case A
    case B
}

@layout(.Packed)
@align(4) // ERROR: '@layout(.Packed)' cannot be combined with '@align': a packed type has alignment 1
struct PackedAligned { var v: lang.i64 }

@layout(.C)
@align(16)
struct CAligned { var v: lang.i64 }

@layout(.Fancy) // ERROR: '.Fancy' is not a layout; expected '.C' or '.Packed'
struct Unknown { var v: lang.i64 }
