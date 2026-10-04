# kestrel-syntax-tree

`SyntaxKind`, the rowan `Language` impl, and CST helpers.

## Generated code: `kinds.txt` and `kestrel.ungram`

Three files are generated and checked in; never edit them by hand:

| Source | Generated | Contents |
|--------|-----------|----------|
| `kinds.txt` | `src/generated/kinds.rs` | `SyntaxKind`, `SyntaxKind::ALL`, `From<Token>` |
| `kestrel.ungram` | `src/ast/generated.rs` | typed views (one struct per node, one enum per union) |
| `kestrel.ungram` | `src/validate/generated.rs` | each node's rule, for `validate::validate` |

After editing a source, run
`UPDATE_GENERATED=1 cargo test -p kestrel-syntax-tree --test sourcegen`; the
same test without the variable fails while any generated file is stale.
Hand-written conveniences on the views (the `Has*` traits, text helpers) go in
`src/ast/ext.rs`.

`kestrel.ungram` is the single source of truth for tree **shape**: the parser
must build trees that conform (`lib/kestrel-parser/tests/conformance.rs` checks
the whole corpus; debug builds of the parser check every error-free parse), and
CST readers go through the typed views instead of matching child kinds by hand.
A shape change is an edit to the grammar first, then the parser, then the
readers the regenerated views break.

## Adding a `SyntaxKind`

**Append at the end of `kinds.txt`. Never insert mid-file.** rowan green trees
store kinds as raw `u16` discriminants, so inserting shifts every later kind
and corrupts cached trees. Then regenerate (above). A node kind with a shape
also needs a rule in `kestrel.ungram`, or an entry in sourcegen's `UNRULED`
list with the reason it has none.

`syntax_kind_table_round_trips` asserts the generated `ALL` is ordered
(`ALL[n] as u16 == n`), complete, and that an out-of-range raw value still
reads back as `Error`.

### Why there is a table at all

`kind_to_raw` is `kind as u16` and is derived. The inverse needs a table, now
generated from `kinds.txt` with the enum and *proved* by the round-trip test.
It replaced 258 `const NAME: u16` declarations plus 258 match arms over
`raw.0`; because the scrutinee was a `u16`, rustc could not check either list,
and a kind appended without a matching arm silently read back as `Error` — the
*recovery* marker, so the tree looked damaged rather than unknown (F27).

### `__NotAKind`

A `#[doc(hidden)]` end-of-enum marker whose discriminant is the variant count.
It exists only so the round-trip test can prove **completeness**: without it a
truncated `ALL` round-trips happily, since every entry it holds is correct and
the missing kinds are simply never tested. Never construct or emit it — it is
absent from `ALL`, so `kind_from_raw` reads it back as `Error`.

## Predicates over `SyntaxKind` belong here

`is_trivia` and `is_type` live on the enum, not at their call sites. Both had
copies that drifted:

- **trivia** had 9 open-coded `matches!`es plus three duplicate `skip_trivia`
  parsers. The set is owned by `Token::is_trivia` in `kestrel-lexer`;
  `SyntaxKind::is_trivia` is its image under `From<Token>`, and
  `trivia_agrees_with_the_lexer` fails if either side gains a member the other
  lacks. Splitting `///` out of `LineComment` is the scenario it guards.
- **type nodes** had two lists, 14 variants and 12; `TyRef`/`TyMutRef` were
  appended to only one. `every_ty_kind_is_a_type_node` derives the answer from
  the enum — a `Ty*` variant is a type node unless `NON_TYPE_TY_KINDS` excuses
  it with a reason (only `TyList`, which *contains* types).

When you add a predicate over `SyntaxKind`, prefer one whose completeness a test
can derive from `ALL` rather than one that restates a list.
