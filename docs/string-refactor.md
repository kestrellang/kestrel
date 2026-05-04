# String Library Refactor

This document describes a redesign of `std.text` that introduces `StringSlice`, `StringBuilder`, typed string indices, and a `Str` protocol that unifies the read-only API across String and StringSlice. Views (bytes, chars, graphemes, lines, split) become lenses over `StringSlice` rather than raw-pointer wrappers.

## Motivation

The current `std.text` API has several structural problems:

1. **Every slice operation allocates.** `split`, `trimmed`, `substring`, `lines` all copy bytes into a new String. Patterns like "split then inspect first element" are O(n) when they should be O(1).

2. **Unit confusion.** `firstIndex(of:)` returns a byte offset as bare `Int64`. `substring(range:)` consumes char indices. Nothing in the type system prevents mixing them.

3. **Duplicated surface area.** `String.count` duplicates `String.chars.count`. `String.first()` duplicates `String.chars(0)`. Two paths to every answer with no guidance on which to use.

4. **Views don't compose.** `s.chars(0..<5).graphemes` requires an intermediate allocation via `.toString()`. Views are parallel siblings, not a composable hierarchy.

5. **Supporting types are undercooked.** `Char` is not `Formattable` (can't interpolate). `Grapheme` is not `Hash`, `Comparable`, or `Formattable`. No `Char` → `String` conversion path exists.

6. **`appendByte` is public on String.** An unsafe escape hatch that breaks the UTF-8 invariant sits on the primary text type with the same visibility as `append`.

7. **Views hold raw pointers.** No ownership tie to the source string — dangling pointer bugs are possible if the source mutates or is deallocated while a view is live.

## Design Overview

Three types, one protocol, five views, four index types.

```
         StringIndex protocol
         ┌─────────┬──────────┬───────────────┬───────────┐
     ByteIndex  CharIndex  GraphemeIndex  LineIndex
         │         │          │               │
    ┌────┴────┐ ┌──┴───┐ ┌───┴────┐     ┌────┴───┐
    BytesView  CharsView  GraphemesView  LinesView   SplitView
         │         │          │               │          │
         └────┬────┴──────────┴───────────────┴──────────┘
              │
         StringSlice  ← range subscript always returns this
              │
         ┌────┴────┐
       String    Str protocol (shared read API)
         │
    StringBuilder (write-only construction)
```

---

## Part 1: The Three Core Types

### StringSlice

An immutable window into a String's UTF-8 bytes with shared ownership. The central abstraction of the library.

```
struct StringSlice: Str {
    source: RcBox[StringStorage]   // keeps bytes alive via RC
    start: Int64                    // byte offset into source
    end: Int64                      // byte offset, exclusive
}
```

Properties:
- Zero-cost to create from a String (share the RcBox, cover the whole range)
- Zero-cost to narrow (adjust start/end)
- Immutable — no mutating methods
- Keeps the source alive as long as the slice exists
- `.toOwned()` copies just the slice's bytes into an independent String

This is the type that non-mutating operations return and that function parameters accept (via the `Str` protocol).

**Why not `Slice[UInt8]`?** `Slice[T]` is a bare `(pointer, count)` with no ownership tie (see collections refactor). That works for Array because Array mutations are common and any refcount would force constant COW copies. String mutations are rarer, and string views are frequently returned from functions and chained. `StringSlice` uses strong shared ownership (via `RcBox`) to keep the source alive. The two types are cousins with different ownership trade-offs. `StringSlice` can vend a `Slice[UInt8]` via `asBytes()` for FFI.

**Why not a weak reference?** A weak reference would let the source `String` deallocate while slices are still live, producing dangling pointers. That violates the safety goal. The memory-retention trade-off is real, but the fix is `.toOwned()` — it copies the slice's bytes into a new independent `String`.

**Trade-off:** A StringSlice retains the entire source string's buffer, not just the referenced range. For "extract one word from a 10MB string and keep it forever" this is a memory retention issue. `.toOwned()` is the escape hatch — it copies just the slice's bytes into a new, independent String.

### String

The mutable, owning text type. COW semantics as today, but every read-only method returns `StringSlice` instead of allocating a new String.

```
struct String: Str {
    storage: CowBox[StringStorage]
}
```

String only implements:
- Constructors
- The `asSlice()` kernel method (required by `Str`)
- Mutating methods (`append`, `trim`, `clear`, `lowercase`, `replace`, etc.)

Everything read-only is inherited from the `Str` protocol extension. See Part 2.

### StringBuilder

Write-only buffer for efficient string construction. No COW, no RcBox, no `isUnique` checks.

```
struct StringBuilder {
    ptr: Pointer[UInt8]
    len: Int64
    cap: Int64
}
```

Public API:

```
init()
init(capacity: Int64)

mutating func append[S](s: S) where S: Str
mutating func appendChar(c: Char)
mutating func appendByte(b: UInt8)          // unsafe — confined here, not on String
mutating func appendBytes(ptr: Pointer[UInt8], count: Int64)

func build() -> String     // transfer ownership, zero-copy (wrap buffer in RcBox)
mutating func clear()      // reset len to 0, keep buffer for reuse
```

This is where `appendByte` lives. It leaves String's public API entirely.

`build()` wraps the builder's buffer in an `RcBox[StringStorage]` and returns an owned String without copying. The builder is left in the same state as `StringBuilder()` — zero length, zero capacity, null pointer. Calling `build()` a second time returns the empty string `""`. The builder can be reused after `build()` by appending more content and calling `build()` again; it will allocate a fresh buffer on the next append. This makes the builder a reusable tool for hot loops that produce many strings without repeated constructor overhead.

---

## Part 2: The `Str` Protocol

### Kernel

`Str` requires exactly one method from conformers:

```
protocol Str: Iterable, Equatable, Comparable, Hash, Formattable {
    type Item = Char
    type TargetIterator = StringIterator

    func asSlice() -> StringSlice
}
```

StringSlice implements this as `return self`. String implements it by wrapping its full buffer range.

### Protocol Extension

All read-only methods are defined once in `extend Str`. Both String and StringSlice inherit them automatically:

```
extend Str {
    // Size & capacity
    var byteCount: Int64 { ... }
    var isEmpty: Bool { ... }

    // Views
    var bytes: BytesView { ... }
    var chars: CharsView { ... }
    var graphemes: GraphemesView { ... }
    var lines: LinesView { ... }
    func reversed() -> ReversedCharsView { ... }
    func split(separator: String) -> SplitView { ... }
    func split(matching: (Char) -> Bool) -> SplitWhereView { ... }

    // Searching (byte-level exact match — unit-agnostic)
    func contains[S](substring: S) -> Bool where S: Str { ... }
    func starts[S](with prefix: S) -> Bool where S: Str { ... }
    func ends[S](with suffix: S) -> Bool where S: Str { ... }

    // Trimming → StringSlice (zero allocation)
    func trimmed() -> StringSlice { ... }
    func trimmedStart() -> StringSlice { ... }
    func trimmedEnd() -> StringSlice { ... }
    func trimmed(matching: (Char) -> Bool) -> StringSlice { ... }
    func trimmedStart(matching: (Char) -> Bool) -> StringSlice { ... }
    func trimmedEnd(matching: (Char) -> Bool) -> StringSlice { ... }

    // Transforms that must allocate (output length may differ from input)
    func lowercased() -> String { ... }
    func uppercased() -> String { ... }
    func titlecased() -> String { ... }
    func replaced[S](pattern: S, with replacement: S) -> String where S: Str { ... }
    func repeated(count: Int64) -> String { ... }

    // Case-insensitive comparison
    func equalsCaseInsensitive[S](other: S) -> Bool where S: Str { ... }

    // Padding
    func pad(leading length: Int64, with char: Char) -> String { ... }
    func pad(trailing length: Int64, with char: Char) -> String { ... }

    // Conversion
    func toOwned() -> String { ... }

    // Iteration (code points by default)
    func iter() -> StringIterator { ... }

    // Typed-index subscripts (see Part 4)
    subscript[I](index: I) -> I.Yield where I: StringIndex { ... }
    subscript[I](range: Range[I]) -> StringSlice where I: StringIndex { ... }
    subscript[I](range: ClosedRange[I]) -> StringSlice where I: StringIndex { ... }

    // Protocol conformances
    func equals[S](other: S) -> Bool where S: Str { ... }
    func compare[S](other: S) -> Ordering where S: Str { ... }
    func hash[H](mutating into hasher: H) where H: Hasher { ... }
    func format(options: FormatOptions = FormatOptions.default()) -> String { ... }
}
```

Every implementation body operates on `self.asSlice()`, which gives access to the byte pointer and range. The logic is written once. Adding a new method to the extension gives it to both String and StringSlice automatically.

### What Moves Off of String

| Current String method | New location | Return type change |
|---|---|---|
| `count` | `s.chars.count` | — (removed from String; unit must be explicit) |
| `first()` / `last()` | `s.chars.first()` / `s.chars.last()` | — (removed from String) |
| `firstIndex(of:)` | `s.chars.firstIndex(of:)` or `s.bytes.firstIndex(of:)` | `Int64?` → `CharIndex?` or `ByteIndex?` |
| `lastIndex(of:)` | same | same |
| `firstIndex(matching:)` | `s.chars.firstIndex(matching:)` | `Int64?` → `CharIndex?` |
| `substringBytes(from:to:)` | `s.bytes(start..<end)` | `String` → `StringSlice` |
| `substring(range:)` | `s.chars(range)` | `String` → `StringSlice` |
| `appendByte` | `StringBuilder.appendByte` | — (removed from String) |
| `trimmed()` | `extend Str` (inherited) | `String` → `StringSlice` |
| `split(separator:)` | `extend Str` (inherited) | `SplitIterator` → `SplitView` |

### What Stays on String (Mutating Only)

```
struct String: Str {
    // Kernel
    func asSlice() -> StringSlice

    // Constructors
    init()
    init(capacity: Int64)
    init(stringLiteral: lang.ptr[lang.i8], length: lang.i64)
    init[S](from: S) where S: Str        // copy any Str into owned String
    static func fromUtf8(bytes: Slice[UInt8]) -> String?

    // Mutating methods (String-only)
    mutating func append[S](other: S) where S: Str
    mutating func appendChar(c: Char)
    mutating func clear()
    mutating func trim()
    mutating func trimStart()
    mutating func trimEnd()
    mutating func trim(matching: (Char) -> Bool)
    mutating func trimStart(matching: (Char) -> Bool)
    mutating func trimEnd(matching: (Char) -> Bool)
    mutating func lowercase()
    mutating func uppercase()
    mutating func replace[S](pattern: S, with replacement: S) where S: Str

    // Everything else: inherited from extend Str
}
```

---

## Part 3: Views as Lenses Over StringSlice

Every view holds a `StringSlice` (not a raw pointer). This makes views safe to pass around and return from functions — no dangling pointers.

The critical rule: **subscript a view with a range, get back a StringSlice (not another view).** This is what enables composition.

### BytesView

```
struct BytesView {
    slice: StringSlice
}
```

| Member | Signature | Complexity |
|---|---|---|
| `count` | `var count: Int64` | O(1) |
| `isEmpty` | `var isEmpty: Bool` | O(1) |
| subscript | `(i: Int64) -> UInt8` | O(1) |
| subscript | `(i: ByteIndex) -> UInt8` | O(1) |
| subscript | `(r: Range[Int64]) -> StringSlice` | O(1) |
| subscript | `(r: Range[ByteIndex]) -> StringSlice` | O(1) |
| subscript checked | `(checked i: Int64) -> UInt8?` | O(1) |
| subscript checked | `(checked i: ByteIndex) -> UInt8?` | O(1) |
| `firstIndex(of:)` | `func firstIndex(of: UInt8) -> ByteIndex?` | O(n) |
| `lastIndex(of:)` | `func lastIndex(of: UInt8) -> ByteIndex?` | O(n) |
| `iter()` | `func iter() -> BytesIterator` | yields `UInt8` |
| `asSlice()` | `func asSlice() -> Slice[UInt8]` | O(1), for FFI |
| `startIndex` | `var startIndex: ByteIndex` | O(1) |
| `endIndex` | `var endIndex: ByteIndex` | O(1) |

### CharsView

```
struct CharsView {
    slice: StringSlice
}
```

| Member | Signature | Complexity |
|---|---|---|
| `count` | `var count: Int64` | O(n) |
| `isEmpty` | `var isEmpty: Bool` | O(1) — checks byteCount |
| subscript | `(i: Int64) -> Char` | O(n) |
| subscript | `(i: CharIndex) -> Char` | O(1) |
| subscript | `(r: Range[Int64]) -> StringSlice` | O(n) to resolve endpoints |
| subscript | `(r: Range[CharIndex]) -> StringSlice` | O(1) |
| subscript checked | `(checked i: Int64) -> Char?` | O(n) |
| subscript checked | `(checked i: CharIndex) -> Char?` | O(1) |
| `first()` | `func first() -> Char?` | O(1) |
| `last()` | `func last() -> Char?` | O(n) |
| `firstIndex(of:)` | `func firstIndex(of: Char) -> CharIndex?` | O(n) |
| `firstIndex(matching:)` | `func firstIndex(matching: (Char) -> Bool) -> CharIndex?` | O(n) |
| `lastIndex(of:)` | `func lastIndex(of: Char) -> CharIndex?` | O(n) |
| `lastIndex(matching:)` | `func lastIndex(matching: (Char) -> Bool) -> CharIndex?` | O(n) |
| `iter()` | `func iter() -> CharsIterator` | yields `Char` |
| `indexedIter()` | `func indexedIter() -> IndexedCharsIterator` | yields `(CharIndex, Char)` |
| `index(at:)` | `func index(at: Int64) -> CharIndex` | O(n) — resolve position |
| `startIndex` | `var startIndex: CharIndex` | O(1) |
| `endIndex` | `var endIndex: CharIndex` | O(1) |

### GraphemesView

```
struct GraphemesView {
    slice: StringSlice
}
```

| Member | Signature | Complexity |
|---|---|---|
| `count` | `var count: Int64` | O(n) |
| `isEmpty` | `var isEmpty: Bool` | O(1) |
| subscript | `(i: Int64) -> Grapheme` | O(n) |
| subscript | `(i: GraphemeIndex) -> Grapheme` | O(1)* |
| subscript | `(r: Range[Int64]) -> StringSlice` | O(n) |
| subscript | `(r: Range[GraphemeIndex]) -> StringSlice` | O(1) |
| subscript checked | `(checked i: Int64) -> Grapheme?` | O(n) |
| `first()` | `func first() -> Grapheme?` | O(1)* |
| `last()` | `func last() -> Grapheme?` | O(n) |
| `firstIndex(matching:)` | `func firstIndex(matching: (Grapheme) -> Bool) -> GraphemeIndex?` | O(n) |
| `iter()` | `func iter() -> GraphemesIterator` | yields `Grapheme` |
| `indexedIter()` | `func indexedIter() -> IndexedGraphemesIterator` | yields `(GraphemeIndex, Grapheme)` |
| `index(at:)` | `func index(at: Int64) -> GraphemeIndex` | O(n) |
| `startIndex` | `var startIndex: GraphemeIndex` | O(1) |
| `endIndex` | `var endIndex: GraphemeIndex` | O(1) |

\* O(1) to locate the start byte; reading the grapheme still requires running the UAX #29 segmenter forward from that byte to find the cluster's end. This is bounded by the cluster length (typically 1-7 code points), so effectively constant for practical text.

### LinesView

```
struct LinesView {
    slice: StringSlice
}
```

| Member | Signature | Complexity |
|---|---|---|
| `count` | `var count: Int64` | O(n) |
| `isEmpty` | `var isEmpty: Bool` | O(1) |
| subscript | `(i: Int64) -> StringSlice` | O(n), no terminator |
| subscript | `(i: LineIndex) -> StringSlice` | O(1)*, no terminator |
| subscript | `(r: Range[Int64]) -> StringSlice` | O(n), terminators preserved |
| subscript | `(r: Range[LineIndex]) -> StringSlice` | O(1) |
| subscript checked | `(checked i: Int64) -> StringSlice?` | O(n) |
| `first()` | `func first() -> StringSlice?` | O(n)** |
| `iter()` | `func iter() -> LinesIterator` | yields `StringSlice` |
| `indexedIter()` | `func indexedIter() -> IndexedLinesIterator` | yields `(LineIndex, StringSlice)` |
| `index(at:)` | `func index(at: Int64) -> LineIndex` | O(n) |
| `startIndex` | `var startIndex: LineIndex` | O(1) |
| `endIndex` | `var endIndex: LineIndex` | O(1) |

\* LineIndex stores the byte offset of the line's start. Reading the line requires scanning forward to find the terminator, so technically O(line length). But the scan is bounded by the line, not the string.

\** `first()` scans for the first line terminator, so O(first line length).

### ReversedCharsView

```
struct ReversedCharsView {
    slice: StringSlice
}
```

| Member | Signature | Complexity |
|---|---|---|
| `count` | `var count: Int64` | O(n) |
| `isEmpty` | `var isEmpty: Bool` | O(1) |
| subscript | `(i: Int64) -> Char` | O(n) |
| subscript | `(i: CharIndex) -> Char` | O(1)* |
| subscript | `(r: Range[CharIndex]) -> StringSlice` | O(1) |
| `first()` | `func first() -> Char?` | O(1) |
| `last()` | `func last() -> Char?` | O(n) |
| `iter()` | `func iter() -> ReversedCharsIterator` | yields `Char` back-to-front |
| `startIndex` | `var startIndex: CharIndex` | O(1) |
| `endIndex` | `var endIndex: CharIndex` | O(1) |

\* Reads the char at the byte offset by scanning backward to find the start of the previous UTF-8 sequence.

### SplitView (new)

```
struct SplitView {
    slice: StringSlice
    separator: String
}

struct SplitWhereView {
    slice: StringSlice
    predicate: (Char) -> Bool
}
```

| Member | Signature | Complexity |
|---|---|---|
| `count` | `var count: Int64` | O(n) |
| `isEmpty` | `var isEmpty: Bool` | O(1) |
| `first()` | `func first() -> StringSlice?` | O(n)* |
| `last()` | `func last() -> StringSlice?` | O(n) |
| `iter()` | `func iter() -> SplitIterator` | yields `StringSlice` |
| `collect()` | `func collect() -> Array[StringSlice]` | O(n), materializes all segments |

\* O(separator scan) to find the first match, not O(whole string).

SplitView is lazy — `s.split(",").first()` only scans to the first separator.

No random-access subscript. To access the i-th segment, either iterate or `.collect()` into an array.

### View Composition

Range subscripts return StringSlice, and StringSlice has views. This is what enables chaining:

```
s.lines(2)                        // StringSlice — third line
s.lines(2).split(" ").first()     // StringSlice — first word of third line
s.lines(2).chars(0..<10)          // StringSlice — first 10 chars of third line
s.chars(0..<20).graphemes.count   // Int64 — grapheme count of first 20 code points
```

No intermediate allocation until `.toOwned()` is called. Each step narrows the byte window; the views reinterpret it.

---

## Part 4: Typed String Indices

### The Index Types

Each index wraps a pre-resolved byte offset. The type tag determines what unit the index represents and what the String subscript returns:

```
protocol StringIndex: Equatable, Comparable {
    type Yield
    func read(from: StringSlice) -> Yield
}

struct ByteIndex: StringIndex {
    type Yield = UInt8
    var byteOffset: Int64
}

struct CharIndex: StringIndex {
    type Yield = Char
    var byteOffset: Int64
}

struct GraphemeIndex: StringIndex {
    type Yield = Grapheme
    var byteOffset: Int64
}

struct LineIndex: StringIndex {
    type Yield = StringSlice
    var byteOffset: Int64
}
```

All four have `Equatable` and `Comparable` conformances based on `byteOffset`. This lets them be used as range endpoints: `Range[CharIndex]`, `Range[ByteIndex]`, etc.

### How Indices Are Produced

Views are the index factory. Search and resolution methods return typed indices:

```
// Search
s.chars.firstIndex(of: ',')                → CharIndex?
s.chars.firstIndex { c in c.isDigit() }    → CharIndex?
s.chars.lastIndex(of: ',')                 → CharIndex?
s.bytes.firstIndex(of: 0x2C)              → ByteIndex?
s.graphemes.firstIndex { g in ... }        → GraphemeIndex?
s.lines.firstIndex { line in ... }         → LineIndex?

// Position resolution — O(n) walk, cached for later O(1) use
s.chars.index(at: 3)          → CharIndex
s.graphemes.index(at: 0)      → GraphemeIndex

// Boundaries — O(1)
s.chars.startIndex             → CharIndex    // byte offset 0
s.chars.endIndex               → CharIndex    // byte offset = byteCount
s.bytes.startIndex             → ByteIndex
s.bytes.endIndex               → ByteIndex
```

### How Indices Are Consumed

Typed indices go directly into String/StringSlice subscripts or back into the same view:

```
// On String/StringSlice (via Str protocol extension):
s(charIdx)                  → Char           O(1)
s(byteIdx)                  → UInt8          O(1)
s(graphemeIdx)              → Grapheme       O(1)*
s(lineIdx)                  → StringSlice    O(line length)
s(charIdx1..<charIdx2)      → StringSlice    O(1)
s(byteIdx1..<byteIdx2)      → StringSlice    O(1)

// On views:
s.chars(charIdx)             → Char           O(1)
s.chars(ci1..<ci2)           → StringSlice    O(1)
s.bytes(byteIdx)             → UInt8          O(1)
```

Type safety: `CharIndex` and `ByteIndex` are different types. You cannot pass a `ByteIndex` where a `CharIndex` is expected, and you cannot construct a `Range[CharIndex]` with a `ByteIndex` endpoint. The compiler catches unit mismatches.

### Index Advancement

```
struct ByteIndex {
    func advance(by n: Int64) -> ByteIndex    // O(1) — pure arithmetic
}

struct CharIndex {
    // Must decode UTF-8 to find next char boundary.
    // O(n) in chars advanced, but O(1) for small n (e.g. skipping a delimiter).
    func advance[S](by n: Int64, in s: S) -> CharIndex where S: Str
}

struct GraphemeIndex {
    // Must run UAX #29 segmenter forward.
    func advance[S](by n: Int64, in s: S) -> GraphemeIndex where S: Str
}

struct LineIndex {
    // Must scan for line terminators.
    func advance[S](by n: Int64, in s: S) -> LineIndex where S: Str
}
```

`ByteIndex.advance` is pure arithmetic — no string needed. All others require the source string to decode/scan forward. This is the same constraint Swift's `String.Index` has (`string.index(after: i)` is a method on the string).

### The Search → Slice Pattern

Typed indices make search-then-slice natural and safe:

```
// Find comma, slice around it
if let .Some(i) = s.chars.firstIndex(of: ',') {
    let before = s(s.chars.startIndex..<i)
    let after = s(i.advance(by: 1, in: s)..<s.chars.endIndex)
}

// Find matching delimiters
let open = s.chars.firstIndex(of: '(')
let close = s.chars.lastIndex(of: ')')
if let (.Some(a), .Some(b)) = (open, close) {
    let inside = s(a.advance(by: 1, in: s)..<b)
}

// Byte-level
if let .Some(nul) = s.bytes.firstIndex(of: 0) {
    let prefix = s(s.bytes.startIndex..<nul)
}
```

---

## Part 5: Grapheme as StringSlice Wrapper

Instead of `Array[Char]`, Grapheme holds a StringSlice pointing at the cluster's bytes:

```
struct Grapheme: Equatable, Comparable, Hash, Formattable, Cloneable {
    slice: StringSlice
}
```

| Member | Implementation | Notes |
|---|---|---|
| `charCount` | `slice.chars.count` | no Array allocation |
| `utf8Length` | `slice.byteCount` | O(1) |
| `firstChar()` | `slice.chars.first()!` | returns `Char`, not `Char?` |
| `chars` | `slice.chars` | returns CharsView, no allocation |
| `isAscii()` | `slice.byteCount == 1 and ...` | O(1) |
| `equals(other)` | `slice.equals(other.slice)` | byte comparison |
| `compare(other)` | `slice.compare(other.slice)` | byte comparison |
| `hash(into:)` | `slice.hash(into:)` | hash the bytes |
| `format(options:)` | `slice.format(options:)` | works via Str |
| `toOwned()` | `slice.toOwned()` | copies just the cluster bytes |
| `clone()` | `Grapheme(slice: slice)` | shallow — shares source |

All the missing conformances (`Hash`, `Comparable`, `Formattable`) fall out naturally because StringSlice already has them via the `Str` protocol.

**Trade-off:** Each Grapheme retains the entire source string via the shared RcBox. For long-lived graphemes extracted from large strings, call `.toOwned()` on the grapheme's slice to cut the cord.

---

## Part 6: Char Fixes

### Formattable Conformance

```
extend Char: Formattable {
    func format(options: FormatOptions = FormatOptions.default()) -> String {
        var s = String();
        s.appendChar(self);
        s.format(options)
    }
}
```

This enables `"\{myChar}"` in string interpolation.

### toString

```
extend Char {
    func toString() -> String {
        var s = String();
        s.appendChar(self);
        s
    }
}
```

### Validated Construction

```
struct Char {
    // Replace current unvalidated init
    init?(value: UInt32) {
        if value > 0x10FFFF { return .None }
        if value >= 0xD800 and value <= 0xDFFF { return .None }  // surrogates
        self._value = value
    }

    // Keep the compiler-emitted init unvalidated (literals are always valid)
    init(charLiteral value: lang.i32) { ... }
}
```

### Rename ASCII-Only Classifiers

| Current | Renamed |
|---|---|
| `isAlphabetic()` | `isAsciiLetter()` |
| `isDigit()` | `isAsciiDigit()` |
| `isAlphanumeric()` | `isAsciiAlphanumeric()` |
| `isUppercase()` | `isAsciiUppercase()` |
| `isLowercase()` | `isAsciiLowercase()` |

`isWhitespace()` and `isControl()` keep their names — they're explicitly about ASCII control characters and the naming is unambiguous.

### Fix isWhitespace / trim Disagreement

`trim()` should use the same whitespace set as `isWhitespace()`. Currently `isWhitespace()` includes form feed (`\x0C`) but `trim()` does not strip it. Align both to the same set: `' '`, `'\t'`, `'\n'`, `'\r'`, `'\x0C'`.

---

## Part 7: Fix equalsCaseInsensitive

The current implementation compares `caseFold(a)` against `caseFold(b)` one code point at a time. Since `caseFold` returns only the first code point of a multi-char fold (e.g. `ß` → `s` instead of `ss`), the iterators go out of sync and produce wrong results for strings containing multi-char fold points.

Fix: expand each char's case fold fully before comparing. Either:

**Option A:** Fold both strings to temporary Strings, then compare bytes:
```
func equalsCaseInsensitive[S](other: S) -> Bool where S: Str {
    self.caseFolded().equals(other.caseFolded())
}

func caseFolded() -> String {
    // build a new string with full case folding (multi-char expansions included)
}
```

**Option B:** Walk both strings with inline fold expansion, comparing the expanded sequences in lockstep without allocating. More complex but avoids the allocation.

Option A is simpler and correct. Option B is an optimization for later.

---

## Part 8: Formatting Changes

### Remove AsciiChars

`AsciiChars.space()` is `' '` with extra steps. Since Kestrel has character literals, this struct adds no value. Remove it.

### Remove Byte Type Alias

`public type Byte = UInt8` is defined but used in zero API signatures. Remove it.

### FormatOptions.equals Must Compare All Fields

The current implementation skips `width` and `precision`, which violates `Equatable`'s contract. Two "equal" FormatOptions can produce different output. Fix: compare all fields, or drop the `Equatable` conformance.

---

## Part 9: Default Iteration

`String: Iterable` with `type Item = Char` means `for c in "hello"` iterates code points. This is consistent with Swift and Rust. The grapheme path (`for g in s.graphemes`) requires an explicit view, which is correct — grapheme iteration is more expensive and the user should opt in.

No change here — code-point iteration as default is the right call. But with Grapheme now being `Formattable` and `Hash`, the grapheme path becomes a first-class citizen rather than the undercooked afterthought it is today.

---

## Part 10: Migration Path

This is a breaking redesign. Suggested migration order:

### Phase 1: Foundation (non-breaking)

1. Add `StringSlice` type
2. Add `StringBuilder` type
3. Add `Str` protocol with `asSlice()` kernel
4. Add `extend Str` with protocol extension methods
5. Make String conform to `Str`
6. Make StringSlice conform to `Str`
7. Add typed index types (`ByteIndex`, `CharIndex`, `GraphemeIndex`, `LineIndex`)

### Phase 2: Views (internal refactor)

8. Refactor views to hold `StringSlice` instead of raw pointers
9. Add typed-index subscripts to views
10. Add search methods to views that return typed indices
11. Make view range-subscripts return `StringSlice`
12. Add `SplitView` and `SplitWhereView`

### Phase 3: Char and Grapheme (non-breaking additions)

13. Add `Char: Formattable` extension
14. Add `Char.toString()`
15. Refactor `Grapheme` to hold `StringSlice` instead of `Array[Char]`
16. Add `Grapheme: Comparable, Hash, Formattable`
17. Add validated `Char.init?(value:)`

### Phase 4: Breaking changes

18. Remove `String.count` (force `s.chars.count`)
19. Remove `String.first()` / `String.last()` (force `s.chars.first()`)
20. Remove `String.firstIndex` / `String.lastIndex` (force view-based search)
21. Remove `String.substringBytes` / `String.substring` (force view subscripts)
22. Move `String.appendByte` to `StringBuilder`
23. Remove `String.appendByte` from public API
24. Change `trimmed*` return types from `String` to `StringSlice`
25. Change `split` return types from iterators to views
26. Rename `Char` ASCII classifiers
27. Fix `isWhitespace` / `trim` disagreement
28. Remove `AsciiChars` struct
29. Remove `Byte` type alias
30. Fix `FormatOptions.equals`
31. Fix `equalsCaseInsensitive`

### Phase 5: Cleanup

32. Remove old iterator types that are superseded by views (`SplitIterator` etc.)
33. Remove the 12+ internal index protocols (`BytesIndex`, `CharsIndex`, etc.)
34. Remove `StringIterator` if `CharsIterator` covers the same role (reconcile the two iterators into one)
35. Update stdlib code that uses the old API
36. Update tests
