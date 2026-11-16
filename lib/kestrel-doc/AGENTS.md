# kestrel-doc

Generates the std reference: JSON for the website and markdown for
`docs/stdlib/`. The `stdlib-docs` skill says when to run it;
`docs/architecture.md` says how it works.

- **Signatures come from AST components** (`Callable`, `TypeAnnotation`,
  `TypeParams`, ...), never from slicing source text.
- **One walk, two outputs.** JSON and markdown are rendered from the same
  `ModulePage`s; don't add a path that builds one without the other.
- **Fix rendering here, not in the `.ks` source.** If an item renders badly,
  change `markdown.rs` or `signature.rs`.
- **Extensions are merged into the type they extend**, not rendered as items.
- **Known limitation:** conformances are matched to protocols by short name
  (`build_protocol_index`), which works only because std protocol names are
  unique. A second protocol with the same name would need real resolution.
- `docs/stdlib/` is generated. Don't edit it by hand.
