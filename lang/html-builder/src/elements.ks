module html.builder

// --- Layout ---

public func div(content: () -> Document) -> Document { el("div", content) }
public func div(attrs: Array[Attr], content: () -> Document) -> Document { el("div", attrs, content) }

public func span(content: () -> Document) -> Document { el("span", content) }
public func span(attrs: Array[Attr], content: () -> Document) -> Document { el("span", attrs, content) }

public func section(content: () -> Document) -> Document { el("section", content) }
public func section(attrs: Array[Attr], content: () -> Document) -> Document { el("section", attrs, content) }

public func header(content: () -> Document) -> Document { el("header", content) }
public func header(attrs: Array[Attr], content: () -> Document) -> Document { el("header", attrs, content) }

public func nav(content: () -> Document) -> Document { el("nav", content) }
public func nav(attrs: Array[Attr], content: () -> Document) -> Document { el("nav", attrs, content) }

public func mainEl(content: () -> Document) -> Document { el("main", content) }
public func mainEl(attrs: Array[Attr], content: () -> Document) -> Document { el("main", attrs, content) }

public func footer(content: () -> Document) -> Document { el("footer", content) }
public func footer(attrs: Array[Attr], content: () -> Document) -> Document { el("footer", attrs, content) }

public func aside(content: () -> Document) -> Document { el("aside", content) }
public func aside(attrs: Array[Attr], content: () -> Document) -> Document { el("aside", attrs, content) }

// --- Headings ---

public func h1(content: () -> Document) -> Document { el("h1", content) }
public func h1(attrs: Array[Attr], content: () -> Document) -> Document { el("h1", attrs, content) }

public func h2(content: () -> Document) -> Document { el("h2", content) }
public func h2(attrs: Array[Attr], content: () -> Document) -> Document { el("h2", attrs, content) }

public func h3(content: () -> Document) -> Document { el("h3", content) }
public func h3(attrs: Array[Attr], content: () -> Document) -> Document { el("h3", attrs, content) }

// --- Text ---

public func p(content: () -> Document) -> Document { el("p", content) }
public func p(attrs: Array[Attr], content: () -> Document) -> Document { el("p", attrs, content) }

public func anchor(content: () -> Document) -> Document { el("a", content) }
public func anchor(attrs: Array[Attr], content: () -> Document) -> Document { el("a", attrs, content) }

public func strong(content: () -> Document) -> Document { el("strong", content) }
public func strong(attrs: Array[Attr], content: () -> Document) -> Document { el("strong", attrs, content) }

public func em(content: () -> Document) -> Document { el("em", content) }
public func em(attrs: Array[Attr], content: () -> Document) -> Document { el("em", attrs, content) }

public func small(content: () -> Document) -> Document { el("small", content) }
public func small(attrs: Array[Attr], content: () -> Document) -> Document { el("small", attrs, content) }

public func code(content: () -> Document) -> Document { el("code", content) }
public func code(attrs: Array[Attr], content: () -> Document) -> Document { el("code", attrs, content) }

public func pre(content: () -> Document) -> Document { el("pre", content) }
public func pre(attrs: Array[Attr], content: () -> Document) -> Document { el("pre", attrs, content) }

// --- Form ---

public func form(content: () -> Document) -> Document { el("form", content) }
public func form(attrs: Array[Attr], content: () -> Document) -> Document { el("form", attrs, content) }

public func button(content: () -> Document) -> Document { el("button", content) }
public func button(attrs: Array[Attr], content: () -> Document) -> Document { el("button", attrs, content) }

public func textarea(content: () -> Document) -> Document { el("textarea", content) }
public func textarea(attrs: Array[Attr], content: () -> Document) -> Document { el("textarea", attrs, content) }

public func label(content: () -> Document) -> Document { el("label", content) }
public func label(attrs: Array[Attr], content: () -> Document) -> Document { el("label", attrs, content) }

public func select(content: () -> Document) -> Document { el("select", content) }
public func select(attrs: Array[Attr], content: () -> Document) -> Document { el("select", attrs, content) }

public func option(content: () -> Document) -> Document { el("option", content) }
public func option(attrs: Array[Attr], content: () -> Document) -> Document { el("option", attrs, content) }

// --- List ---

public func ul(content: () -> Document) -> Document { el("ul", content) }
public func ul(attrs: Array[Attr], content: () -> Document) -> Document { el("ul", attrs, content) }

public func ol(content: () -> Document) -> Document { el("ol", content) }
public func ol(attrs: Array[Attr], content: () -> Document) -> Document { el("ol", attrs, content) }

public func li(content: () -> Document) -> Document { el("li", content) }
public func li(attrs: Array[Attr], content: () -> Document) -> Document { el("li", attrs, content) }

// --- Page structure ---

public func htmlDoc(content: () -> Document) -> Document { el("html", content) }
public func htmlDoc(attrs: Array[Attr], content: () -> Document) -> Document { el("html", attrs, content) }

public func headEl(content: () -> Document) -> Document { el("head", content) }

public func bodyEl(content: () -> Document) -> Document { el("body", content) }
public func bodyEl(attrs: Array[Attr], content: () -> Document) -> Document { el("body", attrs, content) }

public func title(content: () -> Document) -> Document { el("title", content) }

public func style(content: () -> Document) -> Document { el("style", content) }

public func script(content: () -> Document) -> Document { el("script", content) }
public func script(attrs: Array[Attr], content: () -> Document) -> Document { el("script", attrs, content) }

// --- Utility ---

public func spacer() -> Document { Document(raw: #"<span class="spacer"></span>"#) }

// --- Void elements ---

public func input(attrs: Array[Attr]) -> Document { vel("input", attrs) }

public func br() -> Document { vel("br") }

public func hr() -> Document { vel("hr") }
public func hr(attrs: Array[Attr]) -> Document { vel("hr", attrs) }

public func img(attrs: Array[Attr]) -> Document { vel("img", attrs) }

public func linkEl(attrs: Array[Attr]) -> Document { vel("link", attrs) }

public func meta(attrs: Array[Attr]) -> Document { vel("meta", attrs) }
