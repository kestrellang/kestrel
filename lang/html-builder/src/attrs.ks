module html.builder

public func cls(name: String) -> Attr {
    var s = String();
    s.append(#" class=""#);
    s.append(escapeHtml(name));
    s.append(char: '"');
    Attr(raw: s)
}

public func id(name: String) -> Attr {
    var s = String();
    s.append(#" id=""#);
    s.append(escapeHtml(name));
    s.append(char: '"');
    Attr(raw: s)
}

public func href(url: String) -> Attr {
    var s = String();
    s.append(#" href=""#);
    s.append(escapeHtml(url));
    s.append(char: '"');
    Attr(raw: s)
}

public func attr(name: String, value: String) -> Attr {
    var s = String();
    s.append(char: ' ');
    s.append(name);
    s.append(#"=""#);
    s.append(escapeHtml(value));
    s.append(char: '"');
    Attr(raw: s)
}

public func boolAttr(name: String) -> Attr {
    var s = String();
    s.append(char: ' ');
    s.append(name);
    Attr(raw: s)
}
