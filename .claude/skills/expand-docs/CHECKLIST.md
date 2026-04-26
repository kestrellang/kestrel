# Stdlib Doc-Expansion Checklist

Run the `expand-docs` skill against each unchecked file. The numeric
types are templated — only the `.ks.template` files need work; the
generated `int8/16/32/64`, `uint8/16/32/64`, `float32/64` files
regenerate from the templates and are intentionally excluded below.

After each file:
1. Verify with `triage 'stdlib.<area>.<file>.*'` (or the closest
   matching pattern).
2. Tick the box.
3. If you discover new conventions worth recording, update
   `SKILL.md`.

## collections/

- [x] `collections/array.ks` — canonical worked example
- [x] `collections/dictionary.ks`
- [x] `collections/set.ks`
- [x] `collections/hashing.ks`

## core/

- [x] `core/protocols.ks` — central protocol definitions; doc the protocols themselves carefully
- [x] `core/bool.ks`
- [x] `core/range.ks`
- [x] `core/ordering.ks`
- [x] `core/comparison.ks`
- [x] `core/arithmetic.ks`
- [x] `core/bitwise.ks`
- [x] `core/logical.ks`
- [x] `core/assign.ks`
- [x] `core/coalesce.ks`
- [x] `core/convertible.ks`
- [x] `core/copy.ks`
- [x] `core/literals.ks`
- [x] `core/error.ks`
- [x] `core/panic.ks`

## num/ (templates only — skip generated `int*.ks` / `uint*.ks` / `float*.ks`)

- [x] `num/integer.ks.template` — covers Int8/16/32/64 + UInt8/16/32/64
- [x] `num/float.ks.template` — covers Float32/64
- [x] `num/numeric.ks` — non-templated numeric protocols/utilities
- [x] `num/random.ks` — RNG types (Lcg64, RandomNumberGenerator)
- [x] `num/libm.ks` — FFI to libm; minimal docs OK on raw bindings

## text/

- [x] `text/string.ks` — large; expect many sections (`# UTF-8`, `# Indexing`)
- [x] `text/char.ks`
- [x] `text/format.ks` — Formattable, FormatOptions
- [x] `text/views.ks` — string views (chars, lines, etc.)
- [x] `text/unicode/case_tables.ks` — light docs; generated lookup tables
- [x] `text/unicode/case_folding.ks` — light docs; generated lookup tables
- [x] `text/unicode/grapheme_tables.ks` — light docs; generated lookup tables

## memory/

- [x] `memory/pointer.ks` — Pointer, RawPointer (also Slice / SliceIterator)
- [x] `memory/layout.ks` — Layout
- [x] `memory/allocator.ks` — SystemAllocator + Allocator protocol
- [x] `memory/buffer.ks` — Buffer (Slice lives in pointer.ks)
- [x] `memory/literal_slice.ks` — LiteralSlice
- [x] `memory/rcbox.ks` — RcBox (refcounted box)

## iter/

- [x] `iter/iterator.ks` — Iterator + Iterable protocols
- [x] `iter/adapters.ks` — map/filter/zip/etc. adapters

## result/

- [x] `result/optional.ks` — Optional[T] / `T?`
- [x] `result/result.ks` — Result[T, E]

## io/

- [x] `io/io.ks`
- [x] `io/read.ks` — Reader protocol
- [x] `io/write.ks` — Writer protocol
- [x] `io/file.ks` — File handle
- [x] `io/stdio.ks` — stdin/stdout/stderr
- [x] `io/error.ks` — IO errors
- [x] `io/libc.ks` — FFI; minimal docs OK

## ffi/

- [x] `ffi/ffi.ks`
- [x] `ffi/cstring.ks` — CString
- [x] `ffi/libc.ks` — FFI; minimal docs OK

## net/

- [x] `net/socket.ks`
- [x] `net/libc.ks` — FFI; minimal docs OK

## os/

- [x] `os/os.ks`
- [x] `os/env.ks` — environment vars
- [x] `os/fs.ks` — filesystem
- [x] `os/proc.ks` — processes
- [x] `os/platform.ks` — platform detection

## Notes

- **`*libc.ks` files**: thin FFI bindings. A one-line `///` per import
  is enough; full Examples blocks are noise here.
- **`unicode/*_tables.ks`**: mostly large generated lookup constants.
  Document the exported lookup function and any public constants;
  skip per-table docs.
- **Templates**: edit the `.ks.template` and don't touch the generated
  `int*.ks` / `uint*.ks` / `float*.ks` outputs. The build regenerates
  them from the templates.
- Pull the generated outputs into the example sections: e.g. the
  integer template's `# Examples` should show `Int64(...)`, `UInt8(...)`,
  not a hypothetical placeholder type.
