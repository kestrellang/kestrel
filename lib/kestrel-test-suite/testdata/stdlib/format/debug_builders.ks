// test: execution
// stdlib: true
// expect-exit: 0

// The debug builders write struct, tuple, list, set and map forms with the
// writer passed per call; `#?` (alternate) puts each entry on its own
// line, and nested builders indent with it.
module Test
import std.text.(Formatter, Formattable, FormatOptions, DebugStruct, DebugTuple, DebugList, DebugSet, DebugMap)

struct Pt: Formattable {
    var x: Int64
    var y: Int64
    func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
        var s = DebugStruct("Pt", options);
        s.field("x", self.x, into: writer);
        s.field("y", self.y, into: writer);
        s.finish(into: writer)
    }
}
struct Line: Formattable {
    var a: Pt
    var b: Pt
    func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
        var s = DebugStruct("Line", options);
        s.field("a", self.a, into: writer);
        s.field("b", self.b, into: writer);
        s.finish(into: writer)
    }
}
struct Empty: Formattable {
    func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
        DebugStruct("Empty", options).finish(into: writer)
    }
}
struct Meters: Formattable {
    var v: Int64
    func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
        var t = DebugTuple("Meters", options);
        t.field(self.v, into: writer);
        t.finish(into: writer)
    }
}
struct Bag: Formattable {
    var items: [Int64]
    func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
        var l = DebugList(options);
        for i in self.items { l.entry(i, into: writer) }
        l.finish(into: writer);
        var s = DebugSet(options);
        for i in self.items { s.entry(i, into: writer) }
        s.finish(into: writer);
        var m = DebugMap(options);
        for i in self.items { m.entry("k", i, into: writer) }
        m.finish(into: writer)
    }
}

func check(got: String, want: String) -> Int64 {
    if got != want { print("got [\(got)]\nwant [\(want)]\n"); 1 } else { 0 }
}

@main
func main() -> Int64 {
    var bad: Int64 = 0;
    let p = Pt(x: 1, y: 2);
    bad = bad + check("\(p)", "Pt(x: 1, y: 2)");
    bad = bad + check("\(p:#?)", "Pt(\n    x: 1,\n    y: 2,\n)");
    let l = Line(a: Pt(x: 1, y: 2), b: Pt(x: 3, y: 4));
    bad = bad + check("\(l)", "Line(a: Pt(x: 1, y: 2), b: Pt(x: 3, y: 4))");
    bad = bad + check("\(l:#?)", "Line(\n    a: Pt(\n        x: 1,\n        y: 2,\n    ),\n    b: Pt(\n        x: 3,\n        y: 4,\n    ),\n)");
    bad = bad + check("\(Empty())", "Empty");
    bad = bad + check("\(Empty():#?)", "Empty");
    bad = bad + check("\(Meters(v: 5))", "Meters(5)");
    bad = bad + check("\(Bag(items: [1, 2]))", "[1, 2]{1, 2}{k: 1, k: 2}");
    bad = bad + check("\(Bag(items: []))", "[]{}{}");
    bad = bad + check("\(Bag(items: [7]):#?)", "[\n    7,\n]{\n    7,\n}{\n    k: 7,\n}");
    bad
}
