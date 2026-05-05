# String Library Refactor — Progress

Tracking implementation of `docs/string-refactor.md`.

## Phase 1: Foundation (non-breaking) ✅

| Step | Description | Status |
|------|-------------|--------|
| 1 | Add `StringSlice` type (`lang/std/text/slice.ks`) | ✅ |
| 2 | Add `StringBuilder` type (`lang/std/text/builder.ks`) | ✅ |
| 3 | Add `Str` protocol with `asSlice()` kernel (`lang/std/text/str.ks`) | ✅ |
| 4 | Add `extend Str` with protocol extension methods | ✅ |
| 5 | Make String conform to `Str` | ✅ |
| 6 | Make StringSlice conform to `Str` | ✅ |
| 7 | Add typed index types (`ByteIndex`, `CharIndex`, `GraphemeIndex`, `LineIndex`) | ✅ |
| — | CowBox[T] (`lang/std/memory/cowbox.ks`) — prerequisite for StringSlice | ✅ |
| — | Expose internal helpers (remove `fileprivate` from string.ks helpers) | ✅ |

## Phase 2: Views (internal refactor) ✅

| Step | Description | Status |
|------|-------------|--------|
| 8 | Refactor views to hold `StringSlice` instead of raw pointers | ✅ (dual-track: slice + cached ptr/length) |
| 9 | Add typed-index subscripts to views | ✅ |
| 10 | Add search methods to views that return typed indices | ✅ |
| 11 | Make view range-subscripts return `StringSlice` | ✅ |
| 12 | Add `SplitView` and `SplitWhereView` | ✅ |
| — | Add `ReversedCharsView` | ✅ |
| — | Add `IndexedCharsIterator` and `IndexedGraphemesIterator` | ✅ |
| — | Add predicate search (`firstIndex(matching:)`, `lastIndex(matching:)`) | ✅ |
| — | Migrate view internals to slice (remove ptr/length fields) | Skipped — ptr/length kept as cached fields for perf |

## Phase 3: Char and Grapheme (non-breaking additions) ✅

| Step | Description | Status |
|------|-------------|--------|
| 13 | Add `Char: Formattable` extension | ✅ |
| 14 | Add `Char.toString()` | ✅ |
| 15 | Refactor `Grapheme` to hold `StringSlice` instead of `Array[Char]` | Skipped — kept Array[Char] representation |
| 16 | Add `Grapheme: Comparable, Hash, Formattable` | ✅ |
| 17 | Add validated `Char.validated(value:) -> Char?` | ✅ |
| — | `Grapheme.firstChar` changed from `func -> Char?` to `var -> Char` | ✅ |
| — | `Grapheme.chars` changed from method to computed property | ✅ |

## Phase 4: Breaking changes ✅

| Step | Description | Status |
|------|-------------|--------|
| 18 | Remove `String.count` (force `s.chars.count`) | ✅ |
| 19 | Remove `String.first()` / `String.last()` (force `s.chars.first()`) | ✅ (added `first()`/`last()` to CharsView) |
| 20 | Remove `String.firstIndex` / `String.lastIndex` (force view-based search) | ✅ (moved to `extend Str` with `ByteIndex?` return) |
| 21 | Remove `String.substringBytes` / `String.substring` (force view subscripts) | ✅ (kept as internal for stdlib use) |
| 22 | Remove `String.appendByte` entirely | ✅ (kept as internal for stdlib use) |
| 24 | Change `trimmed*` return types from `String` to `StringSlice` | ✅ |
| 25 | Change `split` return types from iterators to views | ✅ |
| 26 | Rename `Char` ASCII classifiers (`isAlphabetic` → `isAsciiLetter`, etc.) | ✅ |
| 27 | Fix `isWhitespace` / `trim` disagreement | ✅ (added form feed to trim) |
| 28 | Remove `AsciiChars` struct | ✅ |
| 29 | Remove `Byte` type alias | ✅ |
| 30 | Fix `FormatOptions.equals` (compare all fields) | ✅ (added fill, width, precision) |
| 31 | Fix `equalsCaseInsensitive` | ✅ (restructured to `caseFolded()` pattern) |
| — | Add `contains`, `starts`, `ends`, `firstIndex(of:)`, `lastIndex(of:)` to `extend Str` | ✅ |
| — | Add `CharsView.first()` and `CharsView.last()` | ✅ |

## Phase 5: Cleanup ✅

| Step | Description | Status |
|------|-------------|--------|
| 32 | Remove old iterator types superseded by views (`SplitIterator`, etc.) | ✅ |
| 33 | Remove the 12+ internal index protocols (`BytesIndex`, `CharsIndex`, etc.) | Deferred — requires type-based overloading in the compiler |
| 34 | Reconcile `StringIterator` and `CharsIterator` into one | N/A — `StringIterator` doesn't exist; `CharsIterator` is the sole char iterator |
| 35 | Update stdlib code that uses the old API | ✅ (done in Phase 4) |
| 36 | Update tests | ✅ (done in Phase 4) |

## New Files Created

| File | Contents |
|------|----------|
| `lang/std/memory/cowbox.ks` | CowBox[T] — COW wrapper over RcBox[T] |
| `lang/std/text/slice.ks` | StringSlice, StringIndex protocol, LineIndex, CharIndex/GraphemeIndex Comparable extensions |
| `lang/std/text/builder.ks` | StringBuilder — write-only buffer, zero-copy `build()` |
| `lang/std/text/str.ks` | Str protocol + `extend Str` with shared read-only API + split methods |

## Modified Files

| File | Changes |
|------|---------|
| `lang/std/text/string.ks` | Storage → CowBox, added Str conformance + `asSlice()`, simplified view properties |
| `lang/std/text/views.ks` | Views hold StringSlice, Cloneable, typed-index conformances, search methods, SplitView/SplitWhereView, ReversedCharsView, IndexedChars/GraphemesIterator |
| `lang/std/text/char.ks` | Char: Formattable + toString() + validated(); Grapheme: Comparable + Hash + Formattable; `.chars` property; `.firstChar` property |
