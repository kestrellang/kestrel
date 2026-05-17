---
name: expand-docs
description: Expand the `///` doc comments in a Kestrel stdlib `.ks` file to the project's full doc-comment formula — high-level summary, detail paragraph, optional `# Safety` / `# Errors` sections, fenced `# Examples`, struct/enum-level `# Examples` / `# Representation` / `# Memory Model` / `# Guarantees` sections, and `@name` tags on every `init` and `subscript`. Use when the user asks to "expand docs", "fill in doc comments", "document this stdlib file", or otherwise wants every public item in a `.ks` source file brought up to the stdlib documentation standard.
---

# Expand Stdlib Doc Comments

Apply this skill when bringing the `///` doc comments in a Kestrel stdlib
`.ks` file (typically under `lang/std/`) up to the project's documentation
standard. Every item in the file gets a doc comment that follows a fixed
shape, and every `init`/`subscript` gets a 2-word `@name` tag.

This skill produces *documentation only* — never change the code itself,
including signatures, default values, or whitespace inside function
bodies. If a comment is wrong because the code's behavior has drifted,
flag it; do not silently rewrite the code to match.

## When to use

- "expand the doc comments in `array.ks`"
- "fill in docs for `dictionary.ks`"
- "document this stdlib file to spec"
- after writing a new stdlib type, before merging

Skip for: in-progress feature code where signatures still churn,
non-stdlib `.ks` files (tests, examples), Rust files (`.rs`).

## The formula

### Functions, computed properties, subscripts, initializers, deinitializers

1. **One-line summary** — what the item does, written so a reader can pick
   it up cold. Imperative or declarative voice is fine; pick one and stay
   consistent within the file.
2. **Detail paragraph** — quirks, complexity, alternatives, related
   items. A few sentences. Cross-reference siblings by name (e.g. "the
   non-mutating mirror is `reversed()`", "compare with `chunks(of:)`").
3. **`# Safety`** *(only if the function is unsafe / has UB preconditions)*
   — explain *why* it's unsafe and what the caller must guarantee.
4. **`# Errors`** *(only if the function panics or returns errors)* —
   list each panic message verbatim, and the trigger conditions.
5. **`# Examples`** — fenced code blocks (` ``` `). One example for
   simple functions; multiple examples covering each call shape, each
   error mode, and each interesting edge case for anything richer.

### Structs, enums

1. **One sentence** about what the type *is*.
2. **A few sentences** about what it's used for, what it does, and which
   related types it composes with.
3. **`# Examples`** — fenced code blocks showing real use, not just
   construction.
4. **Major sections** — domain-specific sections that don't fit in the
   above, e.g. `# Indexing`, `# Capacity & Reallocation` for `Array[T]`,
   `# UTF-8` for `String`, `# Hashing` for `Set` / `Dictionary`. Add
   them only when there's genuinely something to say.
5. **`# Representation`** — the in-memory layout (fields, tag bits,
   pointer layout). One short paragraph.
6. **`# Memory Model`** *(only if the type uses reference semantics or
   has non-obvious ownership behavior)* — refcounting, COW, borrow
   relationships, lifetime gotchas. Skip entirely for plain value types.
7. **`# Guarantees`** *(only if the type has invariants the user must
   know about)* — bullet list of properties the type promises. Skip
   if there's nothing non-obvious to call out.

### Type aliases, associated types, fields

A single `///` line is enough. State what the alias resolves to or what
the field stores. Don't pad these out.

## `@name` tags on inits and subscripts

Every `init` and `subscript` gets a `@name` tag on the first line of its
doc comment, before the summary:

```kestrel
/// @name With Capacity
/// Creates an empty array with at least the requested capacity reserved.
///
/// Equivalent to `Array()` followed by ...
```

Rules:
- **Two words maximum**, both Title Case.
- The name describes the *flavor* of the constructor / subscript, not
  its return type.
- Keep names parallel within a family. For subscripts that come in
  default / checked / unchecked / wrapping / clamping variants, use
  `Indexed`, `Checked Index`, `Unchecked Index`, `Wrapping Index`,
  `Clamping Index` and the corresponding `Range`, `Checked Range`,
  `Unchecked Range`, `Clamping Range` for the range-taking versions.
- For initializer overloads, name them after their *primary argument*
  or their *role*: `Empty`, `With Capacity`, `Repeating Value`,
  `From Iterable`, `From Generator`, `Array Literal`, `Literal Bridge`
  (compiler-emitted), `From Storage` / `From Fields` (internal wrappers).

If you cannot find a 2-word name that fits, ask the user — don't pick
something awkward.

## Examples-block conventions

- Always wrap examples in fenced blocks (` ``` `), never indented.
- No language tag is required (the renderer infers Kestrel).
- **Statements end with `;`**, just like real Kestrel code. This applies
  to every line inside a fenced example block — `let`, `var`,
  assignments, method calls, anything statement-shaped — even when
  the line is a one-liner showing a return value. The result
  comment goes after the semicolon:
  ```
  /// [1, 2, 3].first();  // Some(1)
  /// [].first();         // None
  ```
- For panicking calls, show the call and label the panic:
  ```
  /// arr(9);  // PANIC: index out of bounds
  ```
- For multi-step examples, prefer a single block over several blocks:
  ```
  /// var arr = [1, 2];
  /// arr.append(3);  // [1, 2, 3]
  /// arr.pop();      // Some(3); arr is [1, 2]
  ```
- Lines that aren't statements (block headers, declarations like
  `func foo() { ... }`, lines ending in `{` or `}`) keep their
  natural form — don't add a `;` after a brace.

## Cross-references

When the formula calls for "related items", name them in backticks:
`reversed()`, `Array.chunks(of:)`, `Slice[T]`. Don't link with markdown
syntax — these are doc comments, not Markdown documents. The point is
to help a reader navigate; pick the *one or two* most useful neighbours,
not every method on the type.

## Workflow

1. Read the whole file once before editing — section comments and the
   shape of the type matter.
2. Decide whether you're documenting from scratch or revising existing
   docs. If revising, preserve correct existing prose; don't rewrite for
   style.
3. Track progress with `TaskCreate`/`TaskUpdate` per logical section
   (constructors, properties, accessors, mutators, removers, reorderers,
   capacity, iteration, search, predicates, slicing, chunking,
   partition, conformance extensions). Files of this size (1500+ lines)
   typically have 8-15 sections.
4. After each section, sanity-check that you haven't broken parsing by
   running the relevant tests via the `triage` skill (e.g.
   `triage 'stdlib.collections.array.*'`). Pre-existing failures are
   fine — only flag failures introduced by your edits.
5. After all items are documented, do a sweep for:
   - Any `Example:` (old plain-text style) you missed.
   - Any `init` or `subscript` lacking `@name`.
   - Any public field, type alias, or `extend` block without at least a
     one-line `///`.

## Anti-patterns

- **Don't write WHAT the code already says** — `/// Returns the count`
  on a function called `count()` is noise. Document *behavior*,
  *complexity*, *quirks*, *cross-references*.
- **Don't add `Example:` headers** — examples go under `# Examples`
  inside fenced blocks. The plain-text "Example:" form is the legacy
  style this skill replaces.
- **Don't add `# Memory Model`/`# Guarantees` to plain value types** —
  these sections exist to surface non-obvious behavior. If there's
  nothing surprising, omit the section entirely rather than writing
  "value type, no special guarantees."
- **Don't link to PRs, commits, or issue numbers** — doc comments
  outlive that context. If a quirk needs justification, name the
  invariant, not the ticket.
- **Don't rename or refactor while documenting** — if you spot a bug or
  a bad name, mention it to the user; do not change it as a side effect
  of doc work.

## Reference: array.ks

The file `lang/std/collections/array.ks` is the canonical example of
this skill applied end-to-end. Use it as a template when documenting
sibling collection types.
