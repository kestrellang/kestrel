module notes.html

public func cls(name: String) -> String {
    var s = String();
    s.append(" class=\"");
    s.append(escapeHtml(name));
    s.append(char: '"');
    s
}

public func id(name: String) -> String {
    var s = String();
    s.append(" id=\"");
    s.append(escapeHtml(name));
    s.append(char: '"');
    s
}

public func href(url: String) -> String {
    var s = String();
    s.append(" href=\"");
    s.append(escapeHtml(url));
    s.append(char: '"');
    s
}

public func attr(name: String, value: String) -> String {
    var s = String();
    s.append(char: ' ');
    s.append(name);
    s.append("=\"");
    s.append(escapeHtml(value));
    s.append(char: '"');
    s
}

public func boolAttr(name: String) -> String {
    var s = String();
    s.append(char: ' ');
    s.append(name);
    s
}
