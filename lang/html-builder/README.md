# HTML Builder

Type-safe HTML builder for Kestrel. Compose pages from nested element functions and content closures; text and attribute values are HTML-escaped by default. Rendering is O(total size) — fragments are collected as string pieces and assembled in a single pass. Used by the `notes-frontend` example.

## Installation

```toml
[dependencies]
html-builder = { path = "../../lang/html-builder" }
```

Import from the `html.builder` module.

## Key Types

- **Document** - an HTML fragment (`Addable`, `Cloneable`, `Defaultable`)
  - combine with `+`, or `mutating func append(other: Document)`
  - `func render() -> String` - assemble the final HTML
- **Attr** - a single rendered attribute (e.g. ` class="foo"`)

## Building Blocks

- `el(tag, content: () -> Document)` / `el(tag, attrs, content)` - any element
- `wrap(tag, child: Document)` / `wrap(tag, attrs, child)` - element around an *already-built* Document. Prefer this over a content closure when the child comes from an outer-scope value: a closure would capture it, and captured owned values currently leak (closure envs are never dropped).
- `vel(tag)` / `vel(tag, attrs)` - void element (no closing tag)
- `text(s)` - escaped text node; `raw(s)` - unescaped passthrough; `nothing()` - empty fragment

Named element helpers (each with an optional `attrs: Array[Attr]` overload): `div`, `span`, `section`, `header`, `nav`, `mainEl`, `footer`, `aside`, `h1`-`h3`, `p`, `anchor`, `strong`, `em`, `small`, `code`, `pre`, `form`, `button`, `textarea`, `label`, `select`, `option`, `ul`, `ol`, `li`, `htmlDoc`, `headEl`, `bodyEl`, `title`, `style`, `script`, `spacer`; void elements `input`, `br`, `hr`, `img`, `linkEl`, `meta`.

Attribute helpers (values escaped): `cls(name)`, `id(name)`, `href(url)`, `attr(name, value)`, `boolAttr(name)`.

## Usage

A page shell lifted from `examples/notes-frontend/src/ui/layout.ks`:

```kestrel
import html.builder.(
    raw, text, nothing, el, Document, Attr,
    div, htmlDoc, headEl, bodyEl, title, script, meta,
    cls, id, attr
)

public func page(pageTitle: String, content: Document) -> Document {
    raw("<!DOCTYPE html>")
    + htmlDoc([attr("lang", "en")]) {
        headEl {
            meta([attr("charset", "utf-8")])
            + title { text(pageTitle) }
            + script([attr("src", "https://unpkg.com/htmx.org@1.9.10")]) { nothing() }
        }
        + bodyEl { content }
    }
}

public func appShell(pageTitle: String, sidebar: Document, content: Document) -> Document {
    page(pageTitle,
        div([cls("app")]) {
            el("aside", [cls("sidebar"), id("sidebar")]) { sidebar }
            + el("main", [cls("content"), id("content")]) { content }
        }
    )
}
```

Call `.render()` on the final `Document` to produce the HTML string.
