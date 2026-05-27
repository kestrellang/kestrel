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

// A fragment of escaped HTML content.
public struct Document: Addable, Cloneable, Defaultable {
    type Output = Document

    var value: String

    public static var zero: Document { get { Document() } }

    public init() {
        self.value = String();
    }

    public init(capacity: Int64) {
        self.value = String(capacity: capacity);
    }

    // Wraps a pre-rendered HTML string. Internal — callers outside
    // the module must go through text() or raw().
    init(raw value: String) {
        self.value = value;
    }

    public consuming func add(consuming other: Document) -> Document {
        var result = self;
        result.value.append(other.value);
        result
    }

    public mutating func append(other: Document) {
        self.value.append(other.value);
    }

    public func render() -> String { self.value }

    public func clone() -> Document { Document(raw: self.value.clone()) }
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
