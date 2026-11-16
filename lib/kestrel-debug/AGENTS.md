# kestrel-debug

`ktrace!`, the only way to trace compiler internals. See `docs/architecture.md`.

- **Never `eprintln!`, `println!` or an ad-hoc env flag** for tracing. Add
  `ktrace!("<category>", ...)` at the site.
- **The category must be a string literal.** The macro builds its prefix with
  `concat!`, so a variable or `const` doesn't compile.
- **Categories are free-form; there is no list to update.** Reuse an existing
  one when it fits (the listing command is in `docs/architecture.md`).
- The format arguments are only evaluated when the category is on. Put
  expensive work inside the macro call, not in a `let` before it.
