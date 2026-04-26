// Lightweight string templating
//
// Usage:
//     var t = Template()
//     t.set("name", "Alice")
//     t.setInt("count", 42)
//     let html = t.render("<p>Hello {name}, {count} items</p>")
//
// {key} placeholders are replaced with stored values.
// {{ and }} produce literal braces.
// Missing keys produce empty string.
// set() HTML-escapes values; setRaw() does not.

module plume.plume

func escapeHtml(s: String) -> String {
    var out = String();
    var i: Int64 = 0;
    let len = s.byteCount;
    while i < len {
        let b = s.bytes(unchecked: i);
        if b == 38 {        // &
            out.append("&amp;")
        } else if b == 60 { // <
            out.append("&lt;")
        } else if b == 62 { // >
            out.append("&gt;")
        } else if b == 34 { // "
            out.append("&quot;")
        } else if b == 39 { // '
            out.append("&#39;")
        } else {
            out.appendByte(b)
        };
        i = i + 1
    }
    out
}

public struct Template: Cloneable {
    var vars: Dictionary[String, String]

    public init() {
        self.vars = Dictionary[String, String]()
    }

    /// Set a variable with HTML escaping.
    public mutating func put(k: String, v: String) {
        let _ = self.vars.insert(k, escapeHtml(v));
    }

    /// Set a variable without escaping (for pre-built HTML).
    public mutating func setRaw(k: String, v: String) {
        let _ = self.vars.insert(k, v);
    }

    /// Set an integer variable (no escaping needed).
    public mutating func setInt(k: String, v: Int64) {
        let _ = self.vars.insert(k, v.format());
    }

    /// Remove a variable.
    public mutating func unset(k: String) {
        let _ = self.vars.remove(k);
    }

    /// Remove all variables.
    public mutating func clear() {
        self.vars = Dictionary[String, String]()
    }

    /// Render a pattern string, replacing {key} with stored values.
    /// {{ produces {, }} produces }. Missing keys produce empty string.
    public func render(pattern: String) -> String {
        var out = String();
        let len = pattern.byteCount;
        var i: Int64 = 0;
        var runStart: Int64 = 0;

        while i < len {
            let b = pattern.bytes(unchecked: i);

            if b == 123 { // {
                if i + 1 < len and pattern.bytes(unchecked: i + 1) == 123 {
                    // {{ => literal {
                    if i > runStart {
                        out.append(pattern.substringBytes(from: runStart, to: i))
                    };
                    out.append("{");
                    i = i + 2;
                    runStart = i
                } else {
                    // {key} placeholder
                    if i > runStart {
                        out.append(pattern.substringBytes(from: runStart, to: i))
                    };
                    let keyStart = i + 1;
                    var j = keyStart;
                    while j < len and pattern.bytes(unchecked: j) != 125 {
                        j = j + 1
                    }
                    if j < len {
                        let key = pattern.substringBytes(from: keyStart, to: j);
                        if let .Some(val) = self.vars(key) {
                            out.append(val)
                        };
                        i = j + 1
                    } else {
                        out.append("{");
                        i = keyStart
                    };
                    runStart = i
                }
            } else if b == 125 { // }
                if i + 1 < len and pattern.bytes(unchecked: i + 1) == 125 {
                    // }} => literal }
                    if i > runStart {
                        out.append(pattern.substringBytes(from: runStart, to: i))
                    };
                    out.append("}");
                    i = i + 2;
                    runStart = i
                } else {
                    i = i + 1
                }
            } else {
                i = i + 1
            }
        }

        // Flush remaining literal run
        if runStart < len {
            out.append(pattern.substringBytes(from: runStart, to: len))
        };
        out
    }

    public func clone() -> Template {
        var t = Template();
        t.vars = self.vars.clone();
        t
    }
}
