module notes.html

// --- Layout ---

public func div(content: () -> String) -> String { el("div", content) }
public func div(attrs: Array[String], content: () -> String) -> String { el("div", attrs, content) }

public func span(content: () -> String) -> String { el("span", content) }
public func span(attrs: Array[String], content: () -> String) -> String { el("span", attrs, content) }

public func section(content: () -> String) -> String { el("section", content) }
public func section(attrs: Array[String], content: () -> String) -> String { el("section", attrs, content) }

public func header(content: () -> String) -> String { el("header", content) }
public func header(attrs: Array[String], content: () -> String) -> String { el("header", attrs, content) }

public func nav(content: () -> String) -> String { el("nav", content) }
public func nav(attrs: Array[String], content: () -> String) -> String { el("nav", attrs, content) }

public func mainEl(content: () -> String) -> String { el("main", content) }
public func mainEl(attrs: Array[String], content: () -> String) -> String { el("main", attrs, content) }

public func footer(content: () -> String) -> String { el("footer", content) }
public func footer(attrs: Array[String], content: () -> String) -> String { el("footer", attrs, content) }

public func aside(content: () -> String) -> String { el("aside", content) }
public func aside(attrs: Array[String], content: () -> String) -> String { el("aside", attrs, content) }

// --- Headings ---

public func h1(content: () -> String) -> String { el("h1", content) }
public func h1(attrs: Array[String], content: () -> String) -> String { el("h1", attrs, content) }

public func h2(content: () -> String) -> String { el("h2", content) }
public func h2(attrs: Array[String], content: () -> String) -> String { el("h2", attrs, content) }

public func h3(content: () -> String) -> String { el("h3", content) }
public func h3(attrs: Array[String], content: () -> String) -> String { el("h3", attrs, content) }

// --- Text ---

public func p(content: () -> String) -> String { el("p", content) }
public func p(attrs: Array[String], content: () -> String) -> String { el("p", attrs, content) }

public func anchor(content: () -> String) -> String { el("a", content) }
public func anchor(attrs: Array[String], content: () -> String) -> String { el("a", attrs, content) }

public func strong(content: () -> String) -> String { el("strong", content) }
public func strong(attrs: Array[String], content: () -> String) -> String { el("strong", attrs, content) }

public func em(content: () -> String) -> String { el("em", content) }
public func em(attrs: Array[String], content: () -> String) -> String { el("em", attrs, content) }

public func small(content: () -> String) -> String { el("small", content) }
public func small(attrs: Array[String], content: () -> String) -> String { el("small", attrs, content) }

public func code(content: () -> String) -> String { el("code", content) }
public func code(attrs: Array[String], content: () -> String) -> String { el("code", attrs, content) }

public func pre(content: () -> String) -> String { el("pre", content) }
public func pre(attrs: Array[String], content: () -> String) -> String { el("pre", attrs, content) }

// --- Form ---

public func form(content: () -> String) -> String { el("form", content) }
public func form(attrs: Array[String], content: () -> String) -> String { el("form", attrs, content) }

public func button(content: () -> String) -> String { el("button", content) }
public func button(attrs: Array[String], content: () -> String) -> String { el("button", attrs, content) }

public func textarea(content: () -> String) -> String { el("textarea", content) }
public func textarea(attrs: Array[String], content: () -> String) -> String { el("textarea", attrs, content) }

public func label(content: () -> String) -> String { el("label", content) }
public func label(attrs: Array[String], content: () -> String) -> String { el("label", attrs, content) }

public func select(content: () -> String) -> String { el("select", content) }
public func select(attrs: Array[String], content: () -> String) -> String { el("select", attrs, content) }

public func option(content: () -> String) -> String { el("option", content) }
public func option(attrs: Array[String], content: () -> String) -> String { el("option", attrs, content) }

// --- List ---

public func ul(content: () -> String) -> String { el("ul", content) }
public func ul(attrs: Array[String], content: () -> String) -> String { el("ul", attrs, content) }

public func ol(content: () -> String) -> String { el("ol", content) }
public func ol(attrs: Array[String], content: () -> String) -> String { el("ol", attrs, content) }

public func li(content: () -> String) -> String { el("li", content) }
public func li(attrs: Array[String], content: () -> String) -> String { el("li", attrs, content) }

// --- Page structure ---

public func htmlDoc(content: () -> String) -> String { el("html", content) }
public func htmlDoc(attrs: Array[String], content: () -> String) -> String { el("html", attrs, content) }

public func headEl(content: () -> String) -> String { el("head", content) }

public func bodyEl(content: () -> String) -> String { el("body", content) }
public func bodyEl(attrs: Array[String], content: () -> String) -> String { el("body", attrs, content) }

public func title(content: () -> String) -> String { el("title", content) }

public func style(content: () -> String) -> String { el("style", content) }

public func script(content: () -> String) -> String { el("script", content) }
public func script(attrs: Array[String], content: () -> String) -> String { el("script", attrs, content) }

// --- Utility ---

public func spacer() -> String { "<span class=\"spacer\"></span>" }

// --- Void elements ---

public func input(attrs: Array[String]) -> String { vel("input", attrs) }

public func br() -> String { vel("br") }

public func hr() -> String { vel("hr") }
public func hr(attrs: Array[String]) -> String { vel("hr", attrs) }

public func img(attrs: Array[String]) -> String { vel("img", attrs) }

public func linkEl(attrs: Array[String]) -> String { vel("link", attrs) }

public func meta(attrs: Array[String]) -> String { vel("meta", attrs) }
