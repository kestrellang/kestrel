module html.builder

// Replaces the five HTML-special characters with their entity equivalents.
func escapeHtml(s: String) -> String {
    var out = String();
    for c in s.chars.iter() {
        match c {
            '&' => out.append("&amp;"),
            '<' => out.append("&lt;"),
            '>' => out.append("&gt;"),
            '"' => out.append("&quot;"),
            '\'' => out.append("&#39;"),
            _ => out.append(char: c),
        }
    };
    out
}

// A fragment of HTML content, held as an ordered list of pre-rendered string
// pieces. Building a document (el/vel/+/append) only moves piece *handles*
// between lists — no piece's bytes are copied until render() assembles them
// into one buffer in a single pass. This makes a deeply nested page O(total
// size); the previous single-String representation built bottom-up via
// "<tag>" + inner + "</tag>", which re-copied the accumulated inner HTML at
// every nesting level — O(depth × size).
public struct Document: Addable, Cloneable, Defaultable {
    type Output = Document

    var parts: Array[String]

    public static var zero: Document { get { Document() } }

    public init() {
        self.parts = Array[String]();
    }

    public init(capacity: Int64) {
        // `capacity` was a byte hint for the old single-String storage. With the
        // fragment list the byte buffer is sized once, in render(); kept only for
        // API compatibility.
        self.parts = Array[String]();
    }

    // Wraps a single pre-rendered HTML fragment. Internal — callers outside
    // the module must go through text() or raw().
    init(raw value: String) {
        self.parts = Array[String]();
        self.parts.append(value);
    }

    public consuming func add(consuming other: Document) -> Document {
        var result = self;
        result.append(other);
        result
    }

    public mutating func append(other: Document) {
        self.parts.append(contentsOf: other.parts);
    }

    // Assembles every fragment into one buffer, sized exactly once.
    public func render() -> String {
        var total: Int64 = 0;
        for p in self.parts.iter() {
            total = total + p.byteCount
        };
        var out = String(capacity: total);
        for p in self.parts.iter() {
            out.append(p)
        };
        out
    }

    public func clone() -> Document {
        var copy = Document();
        copy.parts = self.parts.clone();
        copy
    }
}

// A single HTML attribute (e.g. ` class="foo"`).
public struct Attr: Cloneable {
    var value: String

    init(raw value: String) {
        self.value = value;
    }

    public func render() -> String { self.value }

    public func clone() -> Attr { Attr(raw: self.value.clone()) }
}
