# Naming Conventions

## 1. Casing

- Types, structs, enums, protocols, enum variants: `UpperCamelCase`
- Everything else (methods, properties, variables, constants, modules): `lowerCamelCase`
- Generic type parameters: single uppercase letter (`T`, `U`, `E`)
- No underscores in public APIs. `_lowerCamelCase` for private fields and `fileprivate` helpers only.

```
// Types
Array, Optional, RawPointer, Iterator, Equatable

// Enum variants
.Ok, .Err, .Some, .None, .Empty

// Everything else
flatMap, takeWhile, count, isEmpty, Int64.maxValue
```

## 2. Files and modules

**File names match the primary type: `UpperCamelCase.ks`.** If a file contains `RawPointer`, the file is `RawPointer.ks`. If it contains a group of related items, use an UpperCamelCase topic name: `Protocols.ks`, `Arithmetic.ks`.

```
Array.ks            // contains Array
String.ks           // contains String
RawPointer.ks       // contains RawPointer
Protocols.ks        // contains Equatable, Comparable, Hash, etc.
```

**Folders are lowercase and correspond to modules.** The folder structure IS the module structure.

```
std/
  core/             // module std.core
  text/             // module std.text
    unicode/        // module std.text.unicode
  collections/      // module std.collections
  iter/             // module std.iter
  memory/           // module std.memory
```

**Modules are lowercase dot-separated.** Lowercase distinguishes "where it lives" from "what it is" — in `std.collections.Array`, you can instantly see the boundary.

```
std.core, std.text, std.collections, std.iter
```

**Avoid deep nesting.** Two levels is the norm, three is allowed for genuine sub-domains, four or more is a smell.

```
// Good
std.text
std.text.unicode

// Bad
std.text.unicode.tables.casefolding
```

## 3. No abbreviations in public APIs

Spell it out. No shortcuts, no acronyms (except universally understood ones like UTF8, FFI, IO).

Two-letter initialisms for well-known system domains are allowed in module names: `io`, `fs`, `ffi`.

```
// Good
count, capacity, remaining, pointer, address
std.io, std.os.fs, std.ffi

// Bad
cnt, cap, rem, ptr, addr
std.inputOutput, std.foreignFunctionInterface
```

This applies to method names, properties, labels, type names, and associated types.

## 4. Protocol names describe what conformance means

**Default: `-able` / `-ible`** — the conformer has an ability. Use this whenever it sounds natural.

```
Iterable, Equatable, Comparable, Formattable
Copyable, Cloneable, Defaultable, Convertible
ExpressibleByIntLiteral, ExpressibleByStringLiteral
```

**Bare noun** — the conformer IS that thing.

```
Iterator, Hasher, Allocator, SignedInteger
```

**Short name fallback** — when `-able` would sound awkward, use a short verb, noun, or adjective instead.

```
// -able would sound bad here
BitwiseAnd    (not BitwiseAndable)
LeftShift     (not LeftShiftable)
Negate        (not Negateable)
FFISafe       (not FFISafeable)
Equal         (not Equalable)
Modulo        (not Modulable)
```

The test: say it out loud. If the `-able` form sounds natural, use it. If it sounds clunky, drop to a short name.

**Overload noun** — the protocol exists to let multiple types appear in the same position (operator overloading, subscript index overloading, etc.). The name describes the *role* the conformer plays, not an ability.

```
BytesSubstringIndex     // conformer can be a substring index into BytesView
CharsSubstringIndex     // conformer can be a substring index into CharsView
SignedInteger           // conformer is a signed integer
```

## 5. Associated types are full words describing the role

The name describes what role the type plays in the protocol, not what it is.

```
type Item               // the item produced by iteration
type Output             // the result of an operation
type Element            // the element of a sequence
type Bound              // the bound of a range
type Other              // the other operand (not "Rhs")
type Residual           // the value that propagates on early exit (not "Early")
type TargetIterator     // the iterator this iterable produces (not "Iter")
```

## 6. Type names follow `{Context}{Kind}`

When a type is a specific kind of a broader concept, name it `{Context}{Kind}`.

```
ArrayIterator, StringIterator, DictionaryIterator
ArrayStorage, StringStorage, DictionaryStorage
BytesView, CharsView, GraphemesView, LinesView
IoError, ParseError
DefaultHasher
SystemAllocator
```

## 7. Call sites read like English

This is the governing principle for methods and labels. A call site should read as a short English phrase.

```
replace("foo", with: "bar")            → "replace foo with bar"
fold(from: 0, combining: { a + b })    → "fold from 0 combining a+b"
offset(by: 3)                          → "offset by 3"
chunks(of: 4)                          → "chunks of 4"
isValid(index: 5)                      → "is valid index 5"
isDisjoint(with: other)                → "is disjoint with other"
```

**Method names are verbs.** Boolean queries use `is` + adjective.

```
replace, filter, fold, collect, append, insert, remove
isEmpty, isNull, isSorted(), isValid(index:), isSubset(of:)
```

**Labels are prepositions or participles** — `with:`, `from:`, `by:`, `of:`, `at:`, `matching:`, `combining:`, `mapping:`, `using:`, `byKey:`. Not bare nouns like `transform:`, `action:`, `predicate:`, `element:`.

```
// Good
replace("foo", with: "bar")
shuffle(using: rng)
insert("x", at: 3)
sort(byKey: { it.age })

// Bad
filter(predicate: { it > 0 })
forEach(action: { print(it) })
sort(keyExtractor: { it.age })
```

**Don't duplicate meaning** between method name and label.

```
// Bad — "index" twice
isValidIndex(index: 5)

// Good
isValid(index: 5)
```

**Omit labels when obvious** — trailing closures, side-effect closures, positional integers.

```
forEach { print(it) }
map { it * 2 }
nth(2)
stepBy(3)
```

**Keep method names short; push specialization into labels.**

```
// Good
min(by: { it.age })
String(utf8: bytes)

// Bad
minBy(key: { it.age })
String.fromUtf8(bytes: bytes)
```

**Use `matching:` for all predicate closures.** One label, everywhere.

```
filter(matching: { it > 0 })
all(matching: { it > 0 })
any(matching: { it.isReady })
contains(matching: { it.isExpired })
retain(matching: { it.isActive })
```

## 8. Properties vs methods

Both are fine for any complexity. The distinction is semantic:

- **Properties** — describe state or attributes of the value: `count`, `isEmpty`, `capacity`, `isNull`, `address`
- **Methods** — describe actions or computations that feel like "doing something": `collect()`, `fold()`, `iter()`

State queries must be consistent across types: if `isEmpty` is a property on Array, it must be a property on every type that has it — never a method on one type and a property on another. The same applies to `count`, `capacity`, and any other state attribute.

If a property is O(n) or expensive, document it.

## 9. Prefer enums over boolean parameters

A boolean at the call site is opaque. An enum is self-documenting.

```
// Good — clear at the call site
sort(order: .ascending)
search(sensitivity: .caseInsensitive)

// Bad — what does true mean?
sort(ascending: true)
search(caseSensitive: false)
```

## 10. Static factories are for construction that `init` can't express

Use `init` as the default construction path. Static factory methods are for when the method name adds meaning that an `init` label can't convey — typically returning a specific well-known value rather than building from arguments.

```
// Good — static factory, the method name IS the point
Pointer.nullPointer()
RawPointer.nullPointer()

// Good — init, constructing from arguments
String(utf8: bytes)
Int64(from: someUInt8)
Pointer(to: value)
```

## 11. Mutating = verb, non-mutating = past participle

```
trim()       / trimmed()
reverse()    / reversed()
sort()       / sorted()
shuffle()    / shuffled()
lowercase()  / lowercased()
```

When the operation is naturally a noun and there's no good verb form, use the `form` prefix for the mutating variant.

```
formUnion(with:)                / union(with:)
formIntersection(with:)         / intersection(with:)
formSymmetricDifference(with:)  / symmetricDifference(with:)
```

## 12. `to*` converts (new value), `as*` views (no copy)

The prefix signals cost at the call site.

```
// to* — allocates a new independent value
toArray()
toAddress()

// as* — borrows or reinterprets existing storage, cheap
asPointer()
asSlice()
asRaw()
```

## 13. Cross-library consistency

When two libraries expose the same semantic operation, they must use the same name, label, and declaration form. A user shouldn't have to remember that `isEmpty` is a property on Array but a method on BytesView, or that predicates use `matching:` on Set but are unlabeled on Iterator.

Concrete rules:

- **Predicate closures** always use `matching:` — on every type, in every module.
- **Key-extractor closures** always use `byKey:`.
- **Combining closures** always use `combining:`.
- **Mapping closures** always use `mapping:` (or are unlabeled for trailing closures like `map`, `flatMap`).
- **State queries** (`isEmpty`, `count`, `capacity`) are always properties, never methods.
- **Same concept, same name** — if collections call it `first(matching:)`, iterators call it `first(matching:)` too, not `find`.

```
// Good — consistent across Array, Set, Iterator, String
filter(matching: { it > 0 })
all(matching: { it.isReady })
first(matching: { it.isExpired })
sort(byKey: { it.name })
fold(from: 0, combining: { a + b })

// Bad — different names/labels for the same thing
Array.all(satisfying: ...)   // vs Set.all(matching: ...)
Iterator.find(...)           // vs Array.first(matching: ...)
```
