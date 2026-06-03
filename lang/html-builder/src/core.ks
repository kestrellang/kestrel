module html.builder

// Builds a "<tag attr1 attr2>" open-tag fragment — small (tag name + attrs
// only), independent of the element's content.
func openTag(tag: String, attrs: Array[Attr]) -> String {
    var s = String();
    s.append(char: '<');
    s.append(tag);
    for a in attrs.iter() {
        s.append(a.value)
    };
    s.append(char: '>');
    s
}

func closeTag(tag: String) -> String {
    var s = String();
    s.append("</");
    s.append(tag);
    s.append(char: '>');
    s
}

public func el(tag: String, attrs: Array[Attr], content: () -> Document) -> Document {
    var doc = Document();
    doc.parts.append(openTag(tag, attrs));
    doc.append(content());          // splice the child fragments — no byte copy
    doc.parts.append(closeTag(tag));
    doc
}

public func el(tag: String, content: () -> Document) -> Document {
    var doc = Document();
    var open = String();
    open.append(char: '<');
    open.append(tag);
    open.append(char: '>');
    doc.parts.append(open);
    doc.append(content());
    doc.parts.append(closeTag(tag));
    doc
}

public func vel(tag: String, attrs: Array[Attr]) -> Document {
    Document(raw: openTag(tag, attrs))
}

public func vel(tag: String) -> Document {
    var s = String();
    s.append(char: '<');
    s.append(tag);
    s.append(char: '>');
    Document(raw: s)
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
