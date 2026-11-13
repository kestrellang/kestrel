// test: execution
// stdlib: true
// expect-exit: 0

// `pad` and `padIntegral` are the helpers a `format(into:)` uses to honor
// width: a custom type pads its own text, and numbers zero-fill after
// their sign and prefix whatever the alignment.
module Test
import std.text.(Formatter, Formattable, FormatOptions, StringBuilder)

struct Pt: Formattable {
    var x: Int64
    func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
        writer.pad("Pt(\(self.x))", options)
    }
}
@derive(Formattable)
struct Q { var a: Int64; var b: Float64 }

func check(got: String, want: String) -> Int64 {
    if got != want { print("got [\(got)] want [\(want)]\n"); 1 } else { 0 }
}

@main
func main() -> Int64 {
    var bad: Int64 = 0;
    bad = bad + check("[\(Pt(x: 7):>8)]", "[   Pt(7)]");
    bad = bad + check("[\(Pt(x: 7):<8)]", "[Pt(7)   ]");
    bad = bad + check("[\(Pt(x: 7):^9)]", "[  Pt(7)  ]");
    bad = bad + check("[\("hello":.3)]", "[hel]");
    bad = bad + check("[\("hello":>6.3)]", "[   hel]");
    bad = bad + check("[\(-5:05)]", "[-0005]");
    bad = bad + check("[\(42:#010x)]", "[0x0000002a]");
    bad = bad + check("[\(42:+)]", "[+42]");
    bad = bad + check("[\(42:>6)]", "[    42]");
    bad = bad + check("[\(-1.5:08.2)]", "[-0001.50]");
    bad = bad + check("[\(255.5:x)]", "[0x1.ffp+7]");
    bad = bad + check("[\(-255.5:012x)]", "[-0x001.ffp+7]");
    bad = bad + check("[\(true:>6)]", "[  true]");
    bad = bad + check("[\(Q(a: 1, b: 2.5):>20)]", "[     Q(a: 1, b: 2.5)]");
    var b = StringBuilder();
    var o = FormatOptions();
    o.width = .Some(4);
    o.fill = '0';
    b.padIntegral(nonNegative: false, prefix: "", digits: "5", o);
    bad = bad + check(b.build(), "-005");
    // Zero fill ignores a left alignment.
    bad = bad + check("[\(-5:<05)]", "[-0005]");
    bad
}
