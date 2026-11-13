// Debug builders - DebugStruct, DebugTuple, DebugList, DebugSet and DebugMap.

module std.text

import std.core.(Bool)
import std.text.(String, Char, StringSlice, StringBuilder, Formatter, Formattable, FormatOptions)

// ============================================================================
// DEBUG BUILDERS
// ============================================================================
//
// Rust's `debug_struct` family, with the writer passed to each call (a
// builder holding `&mutating writer` can't hand it to a field's `format`,
// whose writer must be `Static`). Under `alternate` (`"\(x:#?)"`) each entry
// goes on its own line, indented, with a trailing comma.

/// The state the debug builders share: the options entries format with,
/// and whether one has been written.
struct DebugEntries {
    let options: FormatOptions
    var hasEntries: Bool

    init(options: FormatOptions) {
        self.options = options;
        self.hasEntries = false
    }

    /// Starts an entry: `open` before the first, a separator before the rest.
    mutating func begin(open: &StringSlice, mutating into writer: some Formatter) {
        if self.options.alternate {
            if self.hasEntries == false {
                writer.append(open);
                writer.append(char: '\n')
            }
            writer.append("    ")
        } else if self.hasEntries {
            writer.append(", ")
        } else {
            writer.append(open)
        }
        self.hasEntries = true
    }

    /// Writes one value of an entry, indented to the entry's depth when
    /// pretty-printing (a nested value's own lines move in with it).
    func value(value: &some Formattable and not Static and not Sized, mutating into writer: some Formatter) {
        if self.options.alternate == false {
            value.format(into: writer, self.options);
            return
        }
        var text = StringBuilder();
        value.format(into: text, self.options);
        for c in text.build().chars {
            writer.append(char: c);
            if c == '\n' {
                writer.append("    ")
            }
        }
    }

    /// Ends an entry: pretty-printing puts each on its own line.
    func end(mutating into writer: some Formatter) {
        if self.options.alternate {
            writer.append(",\n")
        }
    }

    /// Closes the builder: `close` after the entries, `empty` when there
    /// were none.
    func finish(close: &StringSlice, empty: &StringSlice, mutating into writer: some Formatter) {
        if self.hasEntries {
            writer.append(close)
        } else {
            writer.append(empty)
        }
    }
}

/// Writes a struct-shaped debug form: `Pt(x: 1, y: 2)`, or `Pt` with no
/// fields.
///
/// Pass the options your `format(into:)` received, so `"\(pt:#?)"`
/// pretty-prints and nested values see the same flags.
///
/// # Examples
///
/// ```
/// struct Pt: Formattable {
///     var x: Int64
///     var y: Int64
///     func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
///         var s = DebugStruct("Pt", options);
///         s.field("x", self.x, into: writer);
///         s.field("y", self.y, into: writer);
///         s.finish(into: writer)
///     }
/// }
/// "\(Pt(x: 1, y: 2))";     // "Pt(x: 1, y: 2)"
/// "\(Pt(x: 1, y: 2):#?)";  // "Pt(\n    x: 1,\n    y: 2,\n)"
/// ```
public struct DebugStruct {
    let name: String
    var entries: DebugEntries

    /// Starts the debug form of a struct called `name`.
    public init(name: &StringSlice, options: FormatOptions) {
        self.name = name.toOwned();
        self.entries = DebugEntries(options)
    }

    /// Writes the field `name: value`.
    public mutating func field(name: &StringSlice, value: &some Formattable and not Static and not Sized, mutating into writer: some Formatter) {
        if self.entries.hasEntries == false {
            writer.append(self.name)
        }
        self.entries.begin("(", into: writer);
        writer.append(name);
        writer.append(": ");
        self.entries.value(value, into: writer);
        self.entries.end(into: writer)
    }

    /// Closes the form; with no fields, writes just the name.
    public consuming func finish(mutating into writer: some Formatter) {
        self.entries.finish(")", self.name, into: writer)
    }
}

/// Writes a tuple-shaped debug form: `Wrapper(1, "a")`, or `(1, "a")` with
/// an empty name.
///
/// # Examples
///
/// ```
/// var t = DebugTuple("Meters", options);
/// t.field(self.value, into: writer);
/// t.finish(into: writer);   // "Meters(5)"
/// ```
public struct DebugTuple {
    let name: String
    var entries: DebugEntries

    /// Starts the debug form of a tuple-like value called `name`.
    public init(name: &StringSlice, options: FormatOptions) {
        self.name = name.toOwned();
        self.entries = DebugEntries(options)
    }

    /// Writes the next positional field.
    public mutating func field(value: &some Formattable and not Static and not Sized, mutating into writer: some Formatter) {
        if self.entries.hasEntries == false {
            writer.append(self.name)
        }
        self.entries.begin("(", into: writer);
        self.entries.value(value, into: writer);
        self.entries.end(into: writer)
    }

    /// Closes the form; with no fields, writes just the name.
    public consuming func finish(mutating into writer: some Formatter) {
        self.entries.finish(")", self.name, into: writer)
    }
}

/// Writes a list-shaped debug form: `[1, 2, 3]`.
///
/// # Examples
///
/// ```
/// var l = DebugList(options);
/// for item in self.items { l.entry(item, into: writer) }
/// l.finish(into: writer);   // "[1, 2, 3]"
/// ```
public struct DebugList {
    var entries: DebugEntries

    /// Starts a list.
    public init(options: FormatOptions) {
        self.entries = DebugEntries(options)
    }

    /// Writes the next element.
    public mutating func entry(value: &some Formattable and not Static and not Sized, mutating into writer: some Formatter) {
        self.entries.begin("[", into: writer);
        self.entries.value(value, into: writer);
        self.entries.end(into: writer)
    }

    /// Closes the list; `[]` when empty.
    public consuming func finish(mutating into writer: some Formatter) {
        self.entries.finish("]", "[]", into: writer)
    }
}

/// Writes a set-shaped debug form: `{1, 2, 3}`.
///
/// # Examples
///
/// ```
/// var s = DebugSet(options);
/// for item in self.items { s.entry(item, into: writer) }
/// s.finish(into: writer);   // "{1, 2, 3}"
/// ```
public struct DebugSet {
    var entries: DebugEntries

    /// Starts a set.
    public init(options: FormatOptions) {
        self.entries = DebugEntries(options)
    }

    /// Writes the next element.
    public mutating func entry(value: &some Formattable and not Static and not Sized, mutating into writer: some Formatter) {
        self.entries.begin("{", into: writer);
        self.entries.value(value, into: writer);
        self.entries.end(into: writer)
    }

    /// Closes the set; `{}` when empty.
    public consuming func finish(mutating into writer: some Formatter) {
        self.entries.finish("}", "{}", into: writer)
    }
}

/// Writes a map-shaped debug form: `{a: 1, b: 2}`, as `Dictionary` prints.
///
/// # Examples
///
/// ```
/// var m = DebugMap(options);
/// m.entry("a", 1, into: writer);
/// m.finish(into: writer);   // "{a: 1}"
/// ```
public struct DebugMap {
    var entries: DebugEntries

    /// Starts a map.
    public init(options: FormatOptions) {
        self.entries = DebugEntries(options)
    }

    /// Writes the entry `key: value`.
    public mutating func entry(key: &some Formattable and not Static and not Sized, value: &some Formattable and not Static and not Sized, mutating into writer: some Formatter) {
        self.entries.begin("{", into: writer);
        self.entries.value(key, into: writer);
        writer.append(": ");
        self.entries.value(value, into: writer);
        self.entries.end(into: writer)
    }

    /// Closes the map; `{}` when empty.
    public consuming func finish(mutating into writer: some Formatter) {
        self.entries.finish("}", "{}", into: writer)
    }
}
