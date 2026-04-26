/// Tiny self-contained Kestrel syntax highlighter. We only need to color
/// what shows up in stdlib signatures + small doc snippets — keywords,
/// type names, literals, comments, strings — so a hand-rolled tokenizer
/// avoids pulling in Prism/Shiki for ~10KB of pure styling.

import { Fragment } from "react";

const KEYWORDS = new Set([
  "module",
  "import",
  "public",
  "private",
  "internal",
  "fileprivate",
  "func",
  "let",
  "var",
  "mutating",
  "consuming",
  "static",
  "borrowing",
  "struct",
  "enum",
  "protocol",
  "extension",
  "type",
  "init",
  "deinit",
  "subscript",
  "if",
  "else",
  "guard",
  "match",
  "case",
  "for",
  "in",
  "while",
  "loop",
  "break",
  "continue",
  "return",
  "where",
  "as",
  "is",
  "and",
  "or",
  "not",
  "try",
  "throws",
  "indirect",
]);

const LITERALS = new Set(["true", "false", "nil", "self", "Self"]);

/// Keywords that introduce a declaration whose immediately following
/// identifier is the *name* of that declaration (a function name, type
/// name, enum case, etc.). Highlighter colors that next identifier as a
/// "name" regardless of capitalization.
const DECL_KEYWORDS = new Set([
  "func",
  "init",
  "subscript",
  "struct",
  "enum",
  "protocol",
  "extension",
  "type",
  "module",
  "case",
]);

interface Token {
  kind:
    | "keyword"
    | "literal"
    | "type"
    | "name"
    | "label"
    | "string"
    | "number"
    | "comment"
    | "attr"
    | "punct"
    | "text";
  text: string;
}

function tokenize(source: string): Token[] {
  const out: Token[] = [];
  let i = 0;
  const n = source.length;

  // True after we just emitted a `DECL_KEYWORDS` keyword, until we see
  // the first identifier (which we then mark as the declared name).
  // Whitespace and comments preserve the flag; punctuation, attributes,
  // strings, and numbers clear it.
  let pendingDecl = false;

  // True after `(` or `,` inside a paren-list, until we see the first
  // identifier — which is the parameter label (`element` in
  // `(element: T)` or the first `ptr` in `(ptr ptr: Pointer[T])`).
  let expectingLabel = false;
  let parenDepth = 0;

  while (i < n) {
    const c = source[i];

    // Doc / line comment — preserves pendingDecl.
    if (c === "/" && source[i + 1] === "/") {
      let j = i + 2;
      while (j < n && source[j] !== "\n") j++;
      out.push({ kind: "comment", text: source.slice(i, j) });
      i = j;
      continue;
    }

    // Block comment — preserves pendingDecl.
    if (c === "/" && source[i + 1] === "*") {
      let j = i + 2;
      while (j < n - 1 && !(source[j] === "*" && source[j + 1] === "/")) j++;
      j = Math.min(n, j + 2);
      out.push({ kind: "comment", text: source.slice(i, j) });
      i = j;
      continue;
    }

    // String literal — handles \" but no string interpolation parsing.
    if (c === '"') {
      let j = i + 1;
      while (j < n && source[j] !== '"') {
        if (source[j] === "\\") j++;
        j++;
      }
      j = Math.min(n, j + 1);
      out.push({ kind: "string", text: source.slice(i, j) });
      pendingDecl = false;
      i = j;
      continue;
    }

    // Attribute (@inline, @builtin(...))
    if (c === "@") {
      let j = i + 1;
      while (j < n && /[A-Za-z0-9_]/.test(source[j])) j++;
      out.push({ kind: "attr", text: source.slice(i, j) });
      pendingDecl = false;
      i = j;
      continue;
    }

    // Number literal
    if (/[0-9]/.test(c)) {
      let j = i;
      while (j < n && /[0-9_.]/.test(source[j])) j++;
      out.push({ kind: "number", text: source.slice(i, j) });
      pendingDecl = false;
      i = j;
      continue;
    }

    // Identifier / keyword
    if (/[A-Za-z_]/.test(c)) {
      let j = i;
      while (j < n && /[A-Za-z0-9_]/.test(source[j])) j++;
      const word = source.slice(i, j);
      let kind: Token["kind"];
      if (KEYWORDS.has(word)) {
        kind = "keyword";
        pendingDecl = DECL_KEYWORDS.has(word);
        expectingLabel = false;
      } else if (LITERALS.has(word)) {
        kind = "literal";
        pendingDecl = false;
        expectingLabel = false;
      } else if (pendingDecl) {
        // First identifier after a declaration keyword: this is the
        // *name* the declaration introduces (function, type, case).
        kind = "name";
        pendingDecl = false;
        expectingLabel = false;
      } else if (/^[A-Z]/.test(word)) {
        kind = "type";
        expectingLabel = false;
      } else if (expectingLabel && word !== "_") {
        // First lowercase identifier after `(` or `,` in a paren-list —
        // a parameter label (or combined label/name for `(x: T)`).
        kind = "label";
        expectingLabel = false;
      } else {
        kind = "text";
        expectingLabel = false;
      }
      out.push({ kind, text: word });
      i = j;
      continue;
    }

    // Punctuation / operator chunk — group contiguous symbols so things
    // like `->` and `=>` render as a single span. We also walk through
    // each char to keep `parenDepth` and `expectingLabel` in sync.
    if (/[(){}\[\],.:;<>=!?+\-*/&|^~]/.test(c)) {
      let j = i;
      while (j < n && /[(){}\[\],.:;<>=!?+\-*/&|^~]/.test(source[j])) j++;
      const chunk = source.slice(i, j);
      for (const ch of chunk) {
        if (ch === "(") {
          parenDepth++;
          expectingLabel = true;
        } else if (ch === ")") {
          parenDepth = Math.max(0, parenDepth - 1);
          expectingLabel = false;
        } else if (ch === ",") {
          expectingLabel = parenDepth > 0;
        } else {
          // Any other punctuation between the opener and an identifier
          // (`:`, `->`, `=`, etc.) means the upcoming identifier is no
          // longer in the label slot.
          expectingLabel = false;
        }
      }
      out.push({ kind: "punct", text: chunk });
      pendingDecl = false;
      i = j;
      continue;
    }

    // Whitespace / anything else — preserves pendingDecl.
    let j = i + 1;
    while (j < n && !/[A-Za-z0-9_"@/(){}\[\],.:;<>=!?+\-*&|^~]/.test(source[j]))
      j++;
    out.push({ kind: "text", text: source.slice(i, j) });
    i = j;
  }

  return out;
}

const CLASSES: Record<Token["kind"], string> = {
  keyword: "",
  literal: "text-[var(--color-rust)]",
  type: "text-[#5d8aa8] dark:text-[#8ab4d8]",
  name: "text-[var(--color-rust)]",
  label: "text-[var(--color-forest)]",
  string: "text-[#7a8050] dark:text-[#a8b070]",
  number: "text-[#a06030] dark:text-[#d09060]",
  comment: "text-[var(--color-slate-light)] italic",
  attr: "text-[#7a5a30] dark:text-[#c0a070]",
  punct: "text-[var(--color-slate-light)]",
  text: "",
};

export function HighlightedCode({
  code,
  className = "",
}: {
  code: string;
  className?: string;
}) {
  const tokens = tokenize(code);
  return (
    <code className={className}>
      {tokens.map((t, i) => (
        <Fragment key={i}>
          {t.kind === "text" ? (
            t.text
          ) : (
            <span className={CLASSES[t.kind]}>{t.text}</span>
          )}
        </Fragment>
      ))}
    </code>
  );
}
