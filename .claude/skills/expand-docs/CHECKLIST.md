# Stdlib Doc-Expansion Checklist

Run the `expand-docs` skill against each unchecked file. The numeric
types are templated — only `numeric/integer.ks.template` and `numeric/float.ks.template` need work; the
generated `int8/16/32/64`, `uint8/16/32/64`, `float32/64` files
regenerate from the templates and are intentionally excluded below.

Unchecked files were added or moved since the last pass and haven't been
checked against the formula yet; some may already be fine.

After each file:
1. Verify with `triage 'stdlib.<area>.<file>.*'` (or the closest
   matching pattern).
2. Tick the box.
3. If you discover new conventions worth recording, update
   `SKILL.md`.

## collections/

- [x] `collections/array.ks` — canonical worked example
- [ ] `collections/builder.ks`
- [ ] `collections/deque.ks`
- [x] `collections/dictionary.ks`
- [x] `collections/hashing.ks`
- [ ] `collections/heap.ks`
- [x] `collections/set.ks`
- [ ] `collections/slice.ks`
- [ ] `collections/slice_algorithms.ks`
- [ ] `collections/slice_methods.ks`
- [x] `collections/views.ks` — string views (chars, lines, etc.)

## core/

- [x] `core/arithmetic.ks`
- [x] `core/assign.ks`
- [x] `core/bitwise.ks`
- [x] `core/bool.ks`
- [x] `core/coalesce.ks`
- [x] `core/comparison.ks`
- [x] `core/convertible.ks`
- [x] `core/copy.ks`
- [ ] `core/decays.ks`
- [x] `core/error.ks` — IO errors
- [ ] `core/force_unwrap.ks`
- [ ] `core/indirection.ks`
- [x] `core/literals.ks`
- [x] `core/logical.ks`
- [ ] `core/optional_chain.ks`
- [x] `core/ordering.ks`
- [x] `core/panic.ks`
- [x] `core/protocols.ks` — central protocol definitions; doc the protocols themselves carefully
- [x] `core/range.ks`
- [ ] `core/ref.ks`
- [ ] `core/sized.ks`
- [ ] `core/static.ks`
- [ ] `core/tuple.ks`

## ffi/

- [x] `ffi/cstring.ks` — CString
- [x] `ffi/ffi.ks`
- [x] `ffi/libc.ks` — FFI; minimal docs OK
- [ ] `ffi/nullable_cfunction.ks`

## files/

- [ ] `files/contents.ks`
- [ ] `files/directory.ks`
- [x] `files/file.ks` — File handle
- [ ] `files/files.ks`
- [ ] `files/metadata.ks`
- [ ] `files/native.ks`
- [ ] `files/operations.ks`
- [ ] `files/path.ks`

## iter/

- [x] `iter/adapters.ks` — map/filter/zip/etc. adapters
- [x] `iter/iterator.ks` — Iterator + Iterable protocols

## memory/

- [x] `memory/allocator.ks` — SystemAllocator + Allocator protocol
- [ ] `memory/arena.ks`
- [x] `memory/buffer.ks` — Buffer (Slice lives in pointer.ks)
- [ ] `memory/cowbox.ks`
- [x] `memory/layout.ks` — Layout
- [x] `memory/literal_slice.ks` — LiteralSlice
- [ ] `memory/manually_drop.ks`
- [ ] `memory/ownedbox.ks`
- [x] `memory/pointer.ks` — Pointer, RawPointer (also Slice / SliceIterator)
- [x] `memory/rcbox.ks` — RcBox (refcounted box)
- [ ] `memory/sharedbox.ks`
- [ ] `memory/uniquebox.ks`

## network/

- [ ] `network/address.ks`
- [ ] `network/lookup.ks`
- [ ] `network/native.ks`
- [ ] `network/network.ks`
- [x] `network/socket.ks`
- [ ] `network/tcp.ks`
- [ ] `network/udp.ks`
- [ ] `network/unix.ks`

## numeric/

- [x] `numeric/float.ks.template` — covers Float32/64
- [ ] `numeric/float_digits.ks`
- [x] `numeric/integer.ks.template` — covers Int8/16/32/64 + UInt8/16/32/64
- [x] `numeric/libm.ks` — FFI to libm; minimal docs OK on raw bindings
- [x] `numeric/numeric.ks` — non-templated numeric protocols/utilities
- [x] `numeric/random.ks` — RNG types (Lcg64, RandomNumberGenerator)

## result/

- [x] `result/optional.ks` — Optional[T] / `T?`
- [x] `result/result.ks` — Result[T, E]

## streams/

- [ ] `streams/buffered.ks`
- [x] `streams/copy.ks`
- [ ] `streams/descriptor.ks`
- [x] `streams/error.ks` — IO errors
- [ ] `streams/memory.ks`
- [ ] `streams/native.ks`
- [ ] `streams/readable.ks`
- [ ] `streams/seekable.ks`
- [ ] `streams/standard.ks`
- [ ] `streams/streams.ks`
- [ ] `streams/writable.ks`

## system/

- [ ] `system/environment.ks`
- [ ] `system/exit.ks`
- [ ] `system/exitable.ks`
- [ ] `system/machine.ks`
- [ ] `system/native.ks`
- [x] `system/platform.ks` — platform detection
- [ ] `system/process.ks`
- [ ] `system/system.ks`

## text/

- [ ] `text/builder.ks`
- [x] `text/char.ks`
- [ ] `text/debug.ks`
- [x] `text/format.ks` — Formattable, FormatOptions
- [ ] `text/slice.ks`
- [ ] `text/str.ks`
- [x] `text/string.ks` — large; expect many sections (`# UTF-8`, `# Indexing`)
- [x] `text/unicode/case_folding.ks` — light docs; generated lookup tables
- [x] `text/unicode/case_tables.ks` — light docs; generated lookup tables
- [x] `text/unicode/grapheme_tables.ks` — light docs; generated lookup tables
- [x] `text/views.ks` — string views (chars, lines, etc.)

## time/

- [ ] `time/clock.ks`
- [ ] `time/duration.ks`

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
