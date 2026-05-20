module notes.html

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
    }
    out
}

func joinAttrs(attrs: Array[String]) -> String {
    var s = String();
    for a in attrs.iter() {
        s.append(a)
    }
    s
}

public func el(tag: String, attrs: Array[String], content: () -> String) -> String {
    "<" + tag + joinAttrs(attrs) + ">" + content() + "</" + tag + ">"
}

public func el(tag: String, content: () -> String) -> String {
    "<" + tag + ">" + content() + "</" + tag + ">"
}

public func vel(tag: String, attrs: Array[String]) -> String {
    "<" + tag + joinAttrs(attrs) + ">"
}

public func vel(tag: String) -> String {
    "<" + tag + ">"
}

public func text(s: String) -> String {
    escapeHtml(s)
}

public func raw(s: String) -> String {
    s
}

public func nothing() -> String {
    ""
}
