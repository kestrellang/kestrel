# Stdlib Naming Convention Audit

This document lists violations of `@docs/NAMING_CONVENTIONS.md` found in the
generated stdlib reference at `external/kestrel-website/public/stdlib/`.

## 1. Underscores in public API names

> **Rule:** §1 — No underscores in public APIs. `_lowerCamelCase` for private
> fields and `fileprivate` helpers only.

### Leading underscore (should be private)

| File | Item | Kind |
|------|------|------|
| `std.core.json` | `_ExpressibleByArrayLiteral` | protocol |
| `std.core.json` | `_ExpressibleByDictionaryLiteral` | protocol |
| `std.text.json` | `_appendBytes` | function |

### SCREAMING_SNAKE_CASE (should be lowerCamelCase or UpperCamelCase)

**`std.io.libc.json`**

| Item | Kind |
|------|------|
| `MODE_DEFAULT` | function |
| `O_APPEND` | function |
| `O_CREAT` | function |
| `O_EXCL` | function |
| `O_RDONLY` | function |
| `O_RDWR` | function |
| `O_TRUNC` | function |
| `O_WRONLY` | function |
| `SEEK_CUR` | function |
| `SEEK_END` | function |
| `SEEK_SET` | function |
| `STDERR` | function |
| `STDIN` | function |
| `STDOUT` | function |

**`std.net.libc.json`**

| Item | Kind |
|------|------|
| `AF_INET` | function |
| `INADDR_ANY` | function |
| `IPPROTO_TCP` | function |
| `SOCKADDR_IN_SIZE` | function |
| `SOCK_STREAM` | function |
| `SOL_SOCKET` | function |
| `SO_REUSEADDR` | function |

---

## 2. Abbreviations in public API names

> **Rule:** §3 — No abbreviations in public APIs. Spell it out.
> No shortcuts, no acronyms (except universally understood ones like UTF8, FFI, IO).
> Two-letter initialisms for well-known system domains are allowed in module names:
> `io`, `fs`, `ffi`.

### Module abbreviations

| File | Module path | Issue |
|------|-------------|-------|
| `std.io.libc.json` | `std.io.libc` | `libc` is an abbreviation, not a two-letter initialism |
| `std.io.stdio.json` | `std.io.stdio` | `stdio` is an abbreviation, not a two-letter initialism |
| `std.net.libc.json` | `std.net.libc` | `libc` is an abbreviation, not a two-letter initialism |

### Type / function abbreviations

| File | Item | Kind | Suggested rename |
|------|------|------|------------------|
| `std.io.libc.json` | `Fd` | typealias | `FileDescriptor` |
| `std.memory.json` | `RcBox` | struct | `ReferenceCountedBox` |
| `std.ffi.json` | `CString` | struct | `C` is an abbreviation; however, `CString` is borderline acceptable as a well-known FFI concept |
| `std.text.json` | `fromUtf8` | static func | §10 says construction from arguments should use `init`; should be `String(utf8:)` |

---

## 3. Protocol naming

> **Rule:** §4 — Short name fallback when `-able` would sound awkward.

| File | Item | Issue |
|------|------|-------|
| `std.core.json` | `Negatable` | The convention explicitly gives `Negate` as the correct short-name fallback ("not Negateable"). `Negatable` keeps the awkward suffix. |

---

## 4. `to*` vs `as*` conventions

> **Rule:** §12 — `to*` converts (new value), `as*` views (no copy).

| File | Item | Kind | Issue |
|------|------|------|-------|
| `std.text.json` | `fromUtf8` | static func | §10 says this should be an `init` label (`String(utf8:)`), not a static factory |

---

## Appendix: Items that are **NOT** violations

These are commonly questioned but actually comply with the conventions:

- **`Readable` / `Writable`** — §4, `-able` form: the conformer *has* the ability to be read/written. Correct.
- **`Writable` (not `Writeable`)** — Silent `e` is dropped before `-able` per standard English orthography (`write` → `writable`), same as `lovable`, `movable`, etc.
- **`Negatable`** — While listed above as a violation, this is borderline. The convention explicitly says `Negate` (not `Negateable`) but does not explicitly list `Negatable` as wrong. However, since `Negatable` is an unusual English spelling, `Negate` is cleaner.
