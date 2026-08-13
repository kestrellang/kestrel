# kestrel-syntax-tree

`SyntaxKind`, the rowan `Language` impl, and CST helpers.

## Adding a `SyntaxKind`

**Append at the end. Never insert mid-enum.** rowan green trees store kinds as
raw `u16` discriminants, so inserting shifts every later kind and corrupts
cached trees.

Two places, in this order:

1. the `SyntaxKind` enum, immediately before the `__NotAKind` marker;
2. `SyntaxKind::ALL`, at the end.

`syntax_kind_table_round_trips` fails loudly if you miss the second — it asserts
`ALL` is ordered (`ALL[n] as u16 == n`), complete, and that an out-of-range raw
value still reads back as `Error`.

### Why there is a table at all

`kind_to_raw` is `kind as u16` and is derived. The inverse cannot be, without
`unsafe` or a macro that would swallow the enum's inline comments — so `ALL` is
hand-written and *proved* instead. It replaced 258 `const NAME: u16` declarations
plus 258 match arms over `raw.0`; because the scrutinee was a `u16`, rustc could
not check either list, and a kind appended without a matching arm silently read
back as `Error` — the *recovery* marker, so the tree looked damaged rather than
unknown (F27).

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
