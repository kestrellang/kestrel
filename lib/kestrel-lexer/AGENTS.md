# kestrel-lexer

Source text to tokens. See `docs/architecture.md`.

- **`Token::is_trivia` is the only definition of the trivia set.**
  `SyntaxKind::is_trivia` mirrors it, and a test in kestrel-syntax-tree fails
  if they disagree.
- **Adding a token** means a `Token` variant here and a `token` line
  **appended** to `lib/kestrel-syntax-tree/kinds.txt`, then regenerating (see
  that crate's AGENTS.md). Never insert mid-file.
- **Prefer a contextual keyword to a reserved one.** `ref` and `escaping` lex
  as `Identifier`, and the parser checks them with `nth_is_contextual`, so
  existing code that uses the word as a name keeps working.
- **Strings are lexed once.** The modal driver splits interpolated strings into
  `StringStart ... StringEnd` with ordinary tokens in each hole, and collapses a
  hole-free string back into one `String` token. Nothing downstream re-lexes
  string text; don't add a pass that does.
- **The lexer reports nothing.** An unterminated string is cut at its first
  line break so the rest of the file lexes, and is diagnosed downstream (E707
  in kestrel-analyze, or the parser for an interpolated one).
