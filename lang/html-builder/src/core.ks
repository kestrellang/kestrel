module html.builder

func joinAttrs(attrs: Array[Attr]) -> String {
    var s = String();
    for a in attrs.iter() {
        s.append(a.value)
    };
    s
}

public func el(tag: String, attrs: Array[Attr], content: () -> Document) -> Document {
    Document(raw: "<" + tag + joinAttrs(attrs) + ">" + content().value + "</" + tag + ">")
}

public func el(tag: String, content: () -> Document) -> Document {
    Document(raw: "<" + tag + ">" + content().value + "</" + tag + ">")
}

public func vel(tag: String, attrs: Array[Attr]) -> Document {
    Document(raw: "<" + tag + joinAttrs(attrs) + ">")
}

public func vel(tag: String) -> Document {
    Document(raw: "<" + tag + ">")
}

public func text(s: String) -> Document {
    Document(raw: escapeHtml(s))
}

public func raw(s: String) -> Document {
    Document(raw: s)
}

public func nothing() -> Document {
    Document()
}
