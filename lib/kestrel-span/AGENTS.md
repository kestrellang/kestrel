# kestrel-span

`Span`, `Spanned<T>` and `Name`. Read `docs/architecture.md` for the types.

- **`Spanned<T>` equality and hashing ignore the span.** Two `Name`s with the
  same text at different places are equal. Never use a `Spanned` value as an
  identity for a declaration; resolve to an `Entity` and compare that.
- **Offsets are bytes.** `start..end` indexes the UTF-8 source, never chars.
- **`file_id` is the file's entity index** in the hECS world.
- **Compiler-generated code gets `Span::synthetic(file_id)`** (`start == end == 0`).
  A diagnostic should point at a real span when one exists; give generated
  code the span of the user syntax it came from where you can.
