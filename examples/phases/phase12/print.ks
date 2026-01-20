// Print utilities for phase12 examples
// Showcases: extern C functions, ptr[T] primitive, Printable protocol, wrapper types

module Print

// === Extern C bindings ===
// These call into print_support.c for actual printf-based output

@extern(.C, mangleName: "kestrel_print_string")
func c_print_string(ptr: lang.ptr[I8], len: Int) -> Int

@extern(.C, mangleName: "kestrel_print_int")
func c_print_int(value: Int) -> Int

@extern(.C, mangleName: "kestrel_print_float")
func c_print_float(value: Float) -> Int

@extern(.C, mangleName: "kestrel_print_bool")
func c_print_bool(value: Bool) -> Int

@extern(.C, mangleName: "kestrel_print_newline")
func cPrintNewline(dummy: Int) -> Int

// === Printable protocol ===
// Types conforming to this can be printed

protocol Printable {
    func printValue()
}

// === Wrapper types conforming to Printable ===

struct PrintableString: Printable {
    let value: String

    func printValue() {
        let _: Int = c_print_string(self.value.unsafePtr(), self.value.length());
    }
}

struct PrintableInt: Printable {
    let value: Int

    func printValue() {
        let _: Int = c_print_int(self.value);
    }
}

struct PrintableFloat: Printable {
    let value: Float

    func printValue() {
        let _: Int = c_print_float(self.value);
    }
}

struct PrintableBool: Printable {
    let value: Bool

    func printValue() {
        let _: Int = c_print_bool(self.value);
    }
}

// === Convenience print functions ===

public func printString(s: String) {
    PrintableString(value: s).printValue()
}

public func printInt(i: Int) {
    PrintableInt(value: i).printValue()
}

public func printFloat(f: Float) {
    PrintableFloat(value: f).printValue()
}

public func printBool(b: Bool) {
    PrintableBool(value: b).printValue()
}

// === Convenience printLine functions ===

public func printStringLine(s: String) {
    PrintableString(value: s).printValue();
    let _: Int = cPrintNewline(0);
}

public func printIntLine(i: Int) {
    PrintableInt(value: i).printValue();
    let _: Int = cPrintNewline(0);
}

public func printFloatLine(f: Float) {
    PrintableFloat(value: f).printValue();
    let _: Int = cPrintNewline(0);
}

public func printBoolLine(b: Bool) {
    PrintableBool(value: b).printValue();
    let _: Int = cPrintNewline(0);
}

public func printNewline() {
    let _: Int = cPrintNewline(0);
}
