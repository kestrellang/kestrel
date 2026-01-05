// Printable Protocol Example
// Showcases: @extern(.C) FFI bindings, protocols with required methods,
// extension conformances, and generic functions using protocol constraints.

module PrintExample

import std.ffi.(FFISafe)
import std.memory.pointer.(Pointer)

// =============================================================================
// C FFI Bindings for printf family
// =============================================================================

// printf for format strings - we use specific typed versions since variadic 
// extern functions aren't supported yet

@extern(.C, mangleName: "printf")
func printfStr(format: Pointer[UInt8]) -> Int32 {}

@extern(.C, mangleName: "printf")
func printfInt(format: Pointer[UInt8], value: Int64) -> Int32 {}

@extern(.C, mangleName: "printf")
func printfUInt(format: Pointer[UInt8], value: UInt64) -> Int32 {}

@extern(.C, mangleName: "printf")
func printfFloat(format: Pointer[UInt8], value: Float64) -> Int32 {}

// =============================================================================
// Format String Wrapper Structs
// =============================================================================

// These wrap format strings as FFI-safe structs to pass to printf

struct FmtInt: FFISafe {
    var c0: UInt8  // '%'
    var c1: UInt8  // 'l'
    var c2: UInt8  // 'l'
    var c3: UInt8  // 'd'
    var c4: UInt8  // '\0'
    
    static func make() -> FmtInt {
        FmtInt(c0: 37, c1: 108, c2: 108, c3: 100, c4: 0)
    }
}

struct FmtUInt: FFISafe {
    var c0: UInt8  // '%'
    var c1: UInt8  // 'l'
    var c2: UInt8  // 'l'
    var c3: UInt8  // 'u'
    var c4: UInt8  // '\0'
    
    static func make() -> FmtUInt {
        FmtUInt(c0: 37, c1: 108, c2: 108, c3: 117, c4: 0)
    }
}

struct FmtFloat: FFISafe {
    var c0: UInt8  // '%'
    var c1: UInt8  // 'g'
    var c2: UInt8  // '\0'
    
    static func make() -> FmtFloat {
        FmtFloat(c0: 37, c1: 103, c2: 0)
    }
}

struct FmtTrue: FFISafe {
    var c0: UInt8  // 't'
    var c1: UInt8  // 'r'
    var c2: UInt8  // 'u'
    var c3: UInt8  // 'e'
    var c4: UInt8  // '\0'
    
    static func make() -> FmtTrue {
        FmtTrue(c0: 116, c1: 114, c2: 117, c3: 101, c4: 0)
    }
}

struct FmtFalse: FFISafe {
    var c0: UInt8  // 'f'
    var c1: UInt8  // 'a'
    var c2: UInt8  // 'l'
    var c3: UInt8  // 's'
    var c4: UInt8  // 'e'
    var c5: UInt8  // '\0'
    
    static func make() -> FmtFalse {
        FmtFalse(c0: 102, c1: 97, c2: 108, c3: 115, c4: 101, c5: 0)
    }
}

struct FmtNewline: FFISafe {
    var c0: UInt8  // '\n'
    var c1: UInt8  // '\0'
    
    static func make() -> FmtNewline {
        FmtNewline(c0: 10, c1: 0)
    }
}

struct FmtOpenParen: FFISafe {
    var c0: UInt8  // '('
    var c1: UInt8  // '\0'
    
    static func make() -> FmtOpenParen {
        FmtOpenParen(c0: 40, c1: 0)
    }
}

struct FmtCloseParen: FFISafe {
    var c0: UInt8  // ')'
    var c1: UInt8  // '\0'
    
    static func make() -> FmtCloseParen {
        FmtCloseParen(c0: 41, c1: 0)
    }
}

struct FmtComma: FFISafe {
    var c0: UInt8  // ','
    var c1: UInt8  // ' '
    var c2: UInt8  // '\0'
    
    static func make() -> FmtComma {
        FmtComma(c0: 44, c1: 32, c2: 0)
    }
}

struct FmtRgb: FFISafe {
    var c0: UInt8  // 'r'
    var c1: UInt8  // 'g'
    var c2: UInt8  // 'b'
    var c3: UInt8  // '('
    var c4: UInt8  // '\0'
    
    static func make() -> FmtRgb {
        FmtRgb(c0: 114, c1: 103, c2: 98, c3: 40, c4: 0)
    }
}

struct FmtRectOpen: FFISafe {
    var c0: UInt8   // 'R'
    var c1: UInt8   // 'e'
    var c2: UInt8   // 'c'
    var c3: UInt8   // 't'
    var c4: UInt8   // '['
    var c5: UInt8   // 'a'
    var c6: UInt8   // 't'
    var c7: UInt8   // ':'
    var c8: UInt8   // ' '
    var c9: UInt8   // '\0'
    
    static func make() -> FmtRectOpen {
        FmtRectOpen(c0: 82, c1: 101, c2: 99, c3: 116, c4: 91, c5: 97, c6: 116, c7: 58, c8: 32, c9: 0)
    }
}

struct FmtSizeLabel: FFISafe {
    var c0: UInt8  // ','
    var c1: UInt8  // ' '
    var c2: UInt8  // 's'
    var c3: UInt8  // 'i'
    var c4: UInt8  // 'z'
    var c5: UInt8  // 'e'
    var c6: UInt8  // ':'
    var c7: UInt8  // '\0'
    
    static func make() -> FmtSizeLabel {
        FmtSizeLabel(c0: 44, c1: 32, c2: 115, c3: 105, c4: 122, c5: 101, c6: 58, c7: 0)
    }
}

struct FmtTimes: FFISafe {
    var c0: UInt8  // ' '
    var c1: UInt8  // 'x'
    var c2: UInt8  // ' '
    var c3: UInt8  // '\0'
    
    static func make() -> FmtTimes {
        FmtTimes(c0: 32, c1: 120, c2: 32, c3: 0)
    }
}

struct FmtCloseBracket: FFISafe {
    var c0: UInt8  // ']'
    var c1: UInt8  // '\0'
    
    static func make() -> FmtCloseBracket {
        FmtCloseBracket(c0: 93, c1: 0)
    }
}

struct FmtArrow: FFISafe {
    var c0: UInt8  // ' '
    var c1: UInt8  // '-'
    var c2: UInt8  // '>'
    var c3: UInt8  // ' '
    var c4: UInt8  // '\0'
    
    static func make() -> FmtArrow {
        FmtArrow(c0: 32, c1: 45, c2: 62, c3: 32, c4: 0)
    }
}

// =============================================================================
// Printable Protocol
// =============================================================================

/// Types that can be printed to stdout.
/// Conforming types must implement the `printValue()` method which outputs
/// the value's representation to stdout using printf.
public protocol Printable {
    /// Print this value to stdout (without trailing newline)
    func printValue()
}

// =============================================================================
// Printable Conformances for Primitive Types
// =============================================================================

extend Int: Printable {
    public func printValue() {
        var fmt = FmtInt.make();
        printfInt(format: Pointer(to: ref fmt.c0), value: self as Int64);
    }
}

extend Int8: Printable {
    public func printValue() {
        var fmt = FmtInt.make();
        printfInt(format: Pointer(to: ref fmt.c0), value: self as Int64);
    }
}

extend Int16: Printable {
    public func printValue() {
        var fmt = FmtInt.make();
        printfInt(format: Pointer(to: ref fmt.c0), value: self as Int64);
    }
}

extend Int32: Printable {
    public func printValue() {
        var fmt = FmtInt.make();
        printfInt(format: Pointer(to: ref fmt.c0), value: self as Int64);
    }
}

extend Int64: Printable {
    public func printValue() {
        var fmt = FmtInt.make();
        printfInt(format: Pointer(to: ref fmt.c0), value: self);
    }
}

extend UInt: Printable {
    public func printValue() {
        var fmt = FmtUInt.make();
        printfUInt(format: Pointer(to: ref fmt.c0), value: self as UInt64);
    }
}

extend UInt8: Printable {
    public func printValue() {
        var fmt = FmtUInt.make();
        printfUInt(format: Pointer(to: ref fmt.c0), value: self as UInt64);
    }
}

extend UInt16: Printable {
    public func printValue() {
        var fmt = FmtUInt.make();
        printfUInt(format: Pointer(to: ref fmt.c0), value: self as UInt64);
    }
}

extend UInt32: Printable {
    public func printValue() {
        var fmt = FmtUInt.make();
        printfUInt(format: Pointer(to: ref fmt.c0), value: self as UInt64);
    }
}

extend UInt64: Printable {
    public func printValue() {
        var fmt = FmtUInt.make();
        printfUInt(format: Pointer(to: ref fmt.c0), value: self);
    }
}

extend Float32: Printable {
    public func printValue() {
        var fmt = FmtFloat.make();
        printfFloat(format: Pointer(to: ref fmt.c0), value: self as Float64);
    }
}

extend Float64: Printable {
    public func printValue() {
        var fmt = FmtFloat.make();
        printfFloat(format: Pointer(to: ref fmt.c0), value: self);
    }
}

extend Bool: Printable {
    public func printValue() {
        if self {
            var fmt = FmtTrue.make();
            printfStr(format: Pointer(to: ref fmt.c0));
        } else {
            var fmt = FmtFalse.make();
            printfStr(format: Pointer(to: ref fmt.c0));
        }
    }
}

// =============================================================================
// Generic Print Functions
// =============================================================================

/// Print a value to stdout without a trailing newline.
public func print[T: Printable](value: T) {
    value.printValue();
}

/// Print a value to stdout followed by a newline.
public func printLine[T: Printable](value: T) {
    value.printValue();
    var nl = FmtNewline.make();
    printfStr(format: Pointer(to: ref nl.c0));
}

/// Print just a newline.
public func printLine() {
    var nl = FmtNewline.make();
    printfStr(format: Pointer(to: ref nl.c0));
}

// =============================================================================
// Custom Printable Types Example
// =============================================================================

/// A 2D point that can be printed
struct Point: Printable {
    let x: Float64
    let y: Float64
    
    init(x: Float64, y: Float64) {
        self.x = x;
        self.y = y;
    }
    
    func printValue() {
        // Print "(x, y)" format
        var openParen = FmtOpenParen.make();
        var comma = FmtComma.make();
        var closeParen = FmtCloseParen.make();
        
        printfStr(format: Pointer(to: ref openParen.c0));
        self.x.printValue();
        printfStr(format: Pointer(to: ref comma.c0));
        self.y.printValue();
        printfStr(format: Pointer(to: ref closeParen.c0));
    }
}

/// A color with RGB components
struct Color: Printable {
    let r: UInt8
    let g: UInt8
    let b: UInt8
    
    init(r: UInt8, g: UInt8, b: UInt8) {
        self.r = r;
        self.g = g;
        self.b = b;
    }
    
    static func red() -> Color { Color(r: 255, g: 0, b: 0) }
    static func green() -> Color { Color(r: 0, g: 255, b: 0) }
    static func blue() -> Color { Color(r: 0, g: 0, b: 255) }
    static func white() -> Color { Color(r: 255, g: 255, b: 255) }
    static func black() -> Color { Color(r: 0, g: 0, b: 0) }
    
    func printValue() {
        // Print "rgb(r, g, b)" format
        var rgb = FmtRgb.make();
        var comma = FmtComma.make();
        var closeParen = FmtCloseParen.make();
        
        printfStr(format: Pointer(to: ref rgb.c0));
        self.r.printValue();
        printfStr(format: Pointer(to: ref comma.c0));
        self.g.printValue();
        printfStr(format: Pointer(to: ref comma.c0));
        self.b.printValue();
        printfStr(format: Pointer(to: ref closeParen.c0));
    }
}

/// A rectangle defined by origin and size
struct Rectangle: Printable {
    let origin: Point
    let width: Float64
    let height: Float64
    
    init(x: Float64, y: Float64, width: Float64, height: Float64) {
        self.origin = Point(x: x, y: y);
        self.width = width;
        self.height = height;
    }
    
    func area() -> Float64 {
        self.width * self.height
    }
    
    func printValue() {
        // Print "Rect[at: (x, y), size: w x h]"
        var rectOpen = FmtRectOpen.make();
        var sizeLabel = FmtSizeLabel.make();
        var times = FmtTimes.make();
        var closeBracket = FmtCloseBracket.make();
        
        printfStr(format: Pointer(to: ref rectOpen.c0));
        self.origin.printValue();
        printfStr(format: Pointer(to: ref sizeLabel.c0));
        self.width.printValue();
        printfStr(format: Pointer(to: ref times.c0));
        self.height.printValue();
        printfStr(format: Pointer(to: ref closeBracket.c0));
    }
}

// =============================================================================
// Main - Demonstration
// =============================================================================

func main() {
    // Print primitive types
    printLine(value: 42);
    printLine(value: -17);
    printLine(value: 3.14159);
    printLine(value: true);
    printLine(value: false);
    
    // Print unsigned integers
    let bigNum: UInt64 = 18446744073709551615;
    printLine(value: bigNum);
    
    // Print custom types
    let origin = Point(x: 0.0, y: 0.0);
    printLine(value: origin);
    
    let target = Point(x: 100.5, y: 200.25);
    printLine(value: target);
    
    // Print colors
    let red = Color.red();
    printLine(value: red);
    
    let custom = Color(r: 128, g: 64, b: 255);
    printLine(value: custom);
    
    // Print a rectangle
    let rect = Rectangle(x: 10.0, y: 20.0, width: 100.0, height: 50.0);
    printLine(value: rect);
    
    // Print multiple values on same line
    print(value: target);
    var arrow = FmtArrow.make();
    printfStr(format: Pointer(to: ref arrow.c0));
    printLine(value: rect.area());
}
