/// Lightweight string templating with HTML escaping.
///
/// Store variables in a `Template`, then call `render` with a pattern
/// string containing `{key}` placeholders. Values set with `put` are
/// HTML-escaped automatically; `setRaw` bypasses escaping for pre-built
/// HTML fragments. Missing keys produce the empty string.
///
/// Use `{{` and `}}` in patterns to emit literal braces.
///
/// # Examples
///
/// ```
/// var t = Template();
/// t.put("name", "Alice");
/// t.setInt("count", 42);
/// t.render("<p>Hello {name}, {count} items</p>");
/// // "<p>Hello Alice, 42 items</p>"
/// ```

module plume

/// Replaces the five HTML-significant characters (`& < > " '`) with
/// their named entity equivalents so the result is safe to embed in
/// an HTML attribute or text node.
///
/// # Examples
///
/// ```
/// escapeHtml("<b>hi</b>");  // "&lt;b&gt;hi&lt;/b&gt;"
/// escapeHtml("a & b");      // "a &amp; b"
/// escapeHtml("safe");       // "safe"
/// ```
func escapeHtml(s: String) -> String {
    var out = String();
    for c in s.chars.iter() {
        match c {
            '&' => out.append("&amp;"),
            '<' => out.append("&lt;"),
            '>' => out.append("&gt;"),
            '"' => out.append("&quot;"),
            '\'' => out.append("&#39;"),
            _ => out.appendChar(c),
        }
    }
    out
}

/// A string template engine backed by a `Dictionary[String, String]`
/// of named variables.
///
/// Variables are stored pre-escaped (or raw) and substituted into
/// pattern strings at render time. The template itself is reusable —
/// set variables once, then render as many patterns as you like.
///
/// # Examples
///
/// ```
/// var t = Template();
/// t.put("user", "<script>");
/// t.render("Hello {user}!");
/// // "Hello &lt;script&gt;!"
///
/// t.setRaw("link", "<a href='/'>home</a>");
/// t.render("{link}");
/// // "<a href='/'>home</a>"
/// ```
///
/// # Representation
///
/// A single `Dictionary[String, String]` mapping variable names to
/// their (possibly escaped) string values.
public struct Template: Cloneable {
    /// The backing store of variable bindings.
    var vars: Dictionary[String, String]

    /// @name Empty
    /// Creates a template with no variables set.
    ///
    /// # Examples
    ///
    /// ```
    /// var t = Template();
    /// t.render("{x}");  // "" — no variables set
    /// ```
    public init() {
        self.vars = Dictionary[String, String]()
    }

    /// Stores a variable, HTML-escaping the value.
    ///
    /// If `k` already exists it is overwritten. The five HTML-significant
    /// characters (`& < > " '`) in `v` are replaced with entity
    /// references so the rendered output is safe for embedding in HTML.
    ///
    /// # Examples
    ///
    /// ```
    /// var t = Template();
    /// t.put("name", "O'Reilly & Sons");
    /// t.render("{name}");  // "O&#39;Reilly &amp; Sons"
    /// ```
    public mutating func put(k: String, v: String) {
        self.vars.insert(k, escapeHtml(v));
    }

    /// Stores a variable without any escaping.
    ///
    /// Use this for values that are already safe HTML or that will not
    /// appear in an HTML context. If `k` already exists it is
    /// overwritten.
    ///
    /// # Examples
    ///
    /// ```
    /// var t = Template();
    /// t.setRaw("nav", "<nav>...</nav>");
    /// t.render("{nav}");  // "<nav>...</nav>"
    /// ```
    public mutating func setRaw(k: String, v: String) {
        self.vars.insert(k, v);
    }

    /// Stores an integer variable (no escaping needed).
    ///
    /// Converts `v` to its decimal string representation via `format()`
    /// and stores it under `k`. If `k` already exists it is overwritten.
    ///
    /// # Examples
    ///
    /// ```
    /// var t = Template();
    /// t.setInt("count", 99);
    /// t.render("{count} bottles");  // "99 bottles"
    /// ```
    public mutating func setInt(k: String, v: Int64) {
        self.vars.insert(k, "\(v)");
    }

    /// Removes a single variable by name.
    ///
    /// After removal, any `{k}` placeholder in a rendered pattern will
    /// produce the empty string. No-op if `k` is not present.
    ///
    /// # Examples
    ///
    /// ```
    /// var t = Template();
    /// t.put("x", "hi");
    /// t.unset("x");
    /// t.render("{x}");  // ""
    /// ```
    public mutating func unset(k: String) {
        self.vars.remove(k);
    }

    /// Removes all variables, resetting the template to its initial state.
    ///
    /// # Examples
    ///
    /// ```
    /// var t = Template();
    /// t.put("a", "1");
    /// t.put("b", "2");
    /// t.clear();
    /// t.render("{a}{b}");  // ""
    /// ```
    public mutating func clear() {
        self.vars = Dictionary[String, String]()
    }

    /// Renders a pattern string, substituting `{key}` placeholders with
    /// stored variable values.
    ///
    /// Placeholder syntax:
    ///
    /// - `{key}` — replaced with the value of `key`, or the empty
    ///   string if `key` is not set.
    /// - `{{` — literal `{`.
    /// - `}}` — literal `}`.
    ///
    /// An unclosed `{` (no matching `}` before end of input) is emitted
    /// as a literal `{` followed by whatever characters were scanned.
    ///
    /// # Examples
    ///
    /// ```
    /// var t = Template();
    /// t.put("x", "world");
    /// t.render("hello {x}");      // "hello world"
    /// t.render("{{escaped}}");    // "{escaped}"
    /// t.render("{missing}");      // ""
    /// t.render("no placeholders"); // "no placeholders"
    /// ```
    public func render(pattern: String) -> String {
        var out = String();
        var iter = pattern.chars.iter();
        var next = iter.next();

        while let .Some(c) = next {
            next = iter.next();

            if c != '{' and c != '}' {
                out.appendChar(c);
                continue
            }

            if c == '}' {
                out.appendChar('}');
                if let .Some('}') = next {
                    next = iter.next()
                }
                continue
            }

            // c == '{'
            if let .Some('{') = next {
                out.appendChar('{');
                next = iter.next();
                continue
            }

            // {key} placeholder
            var key = String();
            var closed = false;
            if let .Some('}') = next {
                closed = true
            } else if let .Some(n) = next {
                key.appendChar(n);
                while let .Some(k) = iter.next() {
                    if k == '}' {
                        closed = true;
                        break
                    }
                    key.appendChar(k)
                }
            }

            if closed {
                if let .Some(val) = self.vars(key) {
                    out.append(val)
                }
            } else {
                out.appendChar('{');
                out.append(key)
            }
            next = iter.next()
        }
        out
    }

    /// Returns a deep copy of this template, including all stored variables.
    ///
    /// # Examples
    ///
    /// ```
    /// var t = Template();
    /// t.put("a", "1");
    /// var t2 = t.clone();
    /// t2.put("a", "2");
    /// t.render("{a}");   // "1" — original unchanged
    /// t2.render("{a}");  // "2"
    /// ```
    public func clone() -> Template {
        var t = Template();
        t.vars = self.vars.clone();
        t
    }
}
