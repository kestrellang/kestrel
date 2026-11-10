// test: execution
// stdlib: true
// expect-stdout: 42 hi Pt(7)\n|    42|hi   |Pt(      7)|\n[42] [Pt(7)]\n42 Pt(7)\n42;hi;Pt(4);\n42\nPt(7)\n5\n

// `any Formattable` prints: interpolation, format specs, generic
// `some Formattable` code, `formatted()`, array elements, `println` and a
// direct `format(into:)` call. Its `format(into:)` is the `@existentialOnly`
// default, which forwards to the payload's `formatted(options:)`, so the
// options (width, alignment) still reach the payload: `Pt` hands them to
// its field, exactly as it does unboxed.

module Test

struct Pt: Formattable {
    var x: Int64
    func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
        writer.append("Pt(");
        self.x.format(into: writer, options);
        writer.append(")");
    }
}

func bracketed(v: some Formattable) -> String { "[\(v)]" }

@main
func main() {
    let n: Int64 = 42;
    let s: String = "hi";
    let a: any Formattable = n;
    let b: any Formattable = s;
    let c: any Formattable = Pt(x: 7);
    println("\(a) \(b) \(c)");
    println("|\(a:>6)|\(b:<5)|\(c:>7)|");
    println("\(bracketed(a)) \(bracketed(c))");
    println("\(a.formatted()) \(c.formatted())");
    let xs: Array[any Formattable] = [n, s, Pt(x: 4)];
    for x in xs {
        print("\(x);");
    }
    println("");
    println(a);
    println(c);
    let five: Int64 = 5;
    let d: any Formattable = five;
    var sb = StringBuilder();
    d.format(into: sb);
    println(sb.build());
}
