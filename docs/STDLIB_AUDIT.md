# Stdlib Naming Convention Audit

Audit of `external/kestrel-website/public/stdlib/` against `docs/NAMING_CONVENTIONS.md`.

Last updated: 2026-05-04

---

## Definite Violations

### Rule 8 — State queries must be consistent (property vs method)

| Name | Violation | Where |
|------|-----------|-------|
| `isEmpty` | **Method** on `ExactSizeIterator`, **property** everywhere else (Array, Set, Dict, String, Range, Buffer, Slice, etc.) | `std.iter` |
| `count` | **Consuming method** on `Iterator`, **property** everywhere else | `std.iter` |
| `isSorted` | **Method** on `Array` (zero-arg form), while other state queries like `isEmpty` are properties on Array | `std.collections` |

The Iterator cases are debatable — `count()` consumes the iterator and `isSorted()` is consuming too, so the method form signals side-effect. But `isEmpty()` on `ExactSizeIterator` is not consuming and has no excuse.

### Rule 13 — Same concept, same name across types

| Item | Issue |
|------|-------|
| `Iterator.position(matching:)` vs `Array.firstIndex(matching:)` | Same operation (find index of first match), different names |
| `String.find(matching:)` vs `Array.first(matching:)` / `Iterator.first(matching:)` | `find` on String vs `first` on collections/iterators for predicate search. Rule 13 explicitly calls out `find` as bad: *"Iterator.find(...) vs Array.first(matching:)"* |
| `String.find(String) -> Int64?` | Returns an index, not an element — inconsistent with `first(matching:)` which returns the element. Different operation, but the shared name `find` still clashes |
| `countItems(matching:)` on Array/Dict/Set but **missing** on Iterator | Cross-library gap |

### Rule 7 — Don't duplicate meaning between method name and label

| Item | Issue |
|------|-------|
| `inspect(inspecting: ...)` | Redundant — "inspect inspecting" repeats the concept. Should be unlabeled trailing closure or a different label |

### Rule 7 — Labels should be prepositions or participles

| Label | Method | Issue |
|-------|--------|-------|
| `contentsOf:` | `append(contentsOf:)`, `insert(contentsOf:)` | Noun phrase, not a preposition/participle. `from:` would fit better |
| `first:` / `last:` | `drop(first:)`, `drop(last:)` | Adjectives, not prepositions/participles |
| `unchecked:` | `Buffer.read(unchecked:)`, `Buffer.write(unchecked:)` | Adjective |

### Rule 7 — Keep method names short; push specialization into labels

| Item | Issue | Possible fix |
|------|-------|-------------|
| `countItems(matching:)` | Compound name when `count(matching:)` would suffice | `count(matching:)` |
| `takeIf((T) -> Bool)` on Optional | Compound name | `take(matching:)` |
| `stepBy(Int64)` on Iterator | Compound name | `step(by:)` |
| `mergeFrom` on Dictionary | Compound name; also no non-mutating pair per Rule 11 | `merge(from:)` |

---

## Questionable / Borderline

### Rule 1 — No underscores in public APIs

All libc constants use C-style `SCREAMING_SNAKE_CASE`: `O_RDONLY`, `SEEK_SET`, `AF_INET`, `SOCK_STREAM`, `SOL_SOCKET`, etc. (18 constants across `std.io.libc` and `std.net.libc`). These are FFI passthrough bindings where preserving C naming aids discoverability — arguably intentional, but technically violates rule 1.

### Rule 11 — Mutating = verb, non-mutating = past participle

| Mutating | Non-mutating | Issue |
|----------|-------------|-------|
| `trimStart()` | `trimmedStart()` | Compound name. Could be `trim(from: .start)` / `trimmed(from: .start)` using an enum (also satisfies Rule 9: prefer enums over booleans) |
| `trimEnd()` | `trimmedEnd()` | Same |
| `mergeFrom(...)` | *(none)* | No non-mutating counterpart. `merge` already exists as a different mutating method |

### Naming overlap: `filterMap` vs `compactMap`

Iterator has **both** `filterMap[U]((Item) -> U?)` and `compactMap[T]()` (unwraps optionals). They both produce `FilterMapIterator`. The two names for related operations is potentially confusing — most languages pick one name.

### `notEquals` naming

`notEquals(Self) -> Bool` appears on `Equatable`, `Comparable`, and `Ordering`. Boolean queries should use `is` + adjective per Rule 7, so `isNotEqual(to:)` would be more consistent. However, this is a protocol-required operator implementation, so the naming may be forced by the operator protocol pattern.

### Internal fields named against convention

Iterator adapter structs expose `internal` fields named `predicate`, `transform`, `separator` — these are the exact bare-noun labels Rule 7 forbids. They're `internal` (not public API), so technically not a violation, but they leak through the docs.
