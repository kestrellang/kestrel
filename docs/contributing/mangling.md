# Kestrel Symbol Mangling — v0 Scheme

Every Kestrel function, method, initializer, deinit, static, and closure needs a unique linker symbol. The v0 mangler encodes source-level names, module paths, parameter labels, receiver conventions, and generic instantiations into a flat string that:

1. Is unique for every distinct monomorphized symbol.
2. Is deterministic — same input always produces the same output.
3. Is LL(1) parseable, so it can be demangled without backtracking.
4. Uses only characters legal in linker symbols (alphanumeric + `_`).

The implementation lives in `lib/kestrel-mir/src/mono/mangle.rs` — mangling runs during **monomorphization**, as part of the MIR pass pipeline, not in a codegen backend. The sole public entry point is `mangle_function`; it is called once per instantiation in `lib/kestrel-mir/src/mono/mod.rs`, and the resulting name is what both backends (Cranelift and LLVM) emit.

Because mangling is strictly post-mono, abstract types never reach it: `MirTy::TypeParam` and `MirTy::AssociatedProjection` **panic** in `mangle_type` ("monomorphization bug") rather than having encodings.

## When mangling is bypassed

| Symbol | Linker name | Reason |
|--------|-------------|--------|
| Program entry point | `main` | C runtime expects it. |
| `extern` functions | declared name verbatim | FFI. |
| Static initializer | `__kestrel_init_statics` | Fixed ABI. |

Everything else goes through the mangler.

## Top-level grammar

From the doc comment on `mangle_function`:

```
mangled       = "_K0" path receiver? signature? return? instantiation? self-disambig?
path          = ident                          -- single unqualified name
              | "N" ident+ "E"                 -- nested (qualified) name
ident         = length "_" utf8-bytes          -- byte-length-prefixed
length        = [1-9] [0-9]*                   -- decimal, no leading zeros
receiver      = "r" | "m" | "c"                -- borrow / mutating borrow / consuming
signature     = "Z" param* "E"                 -- omitted entirely when there are no params
param         = ("L" ident)? ("r" | "m")? type -- optional external label, optional
                                               -- borrow-convention marker, then type
return        = "R" type                       -- the current implementation always emits it
instantiation = "I" type+ "E"                  -- generic type args
self-disambig = "S_" type                      -- concrete Self for protocol ext methods
```

The `_` between `length` and bytes makes identifiers that start with a digit (closure indices, for example) unambiguous. The `E` terminator on `N`, `Z`, `I`, `T`, `F`, `C` makes every composite production self-delimiting.

All mangled symbols start with `_K0` — `K` identifies Kestrel, `0` is the version. Future revisions bump the digit.

## Path encoding

Entity names in MIR are dot-separated qualified names (`"std.collections.Array.append"`), built by `qualified_name` in `lib/kestrel-mir-lower/src/name.rs` from the entity's parent chain (nameless `init` / `subscript` / `deinit` entities contribute fixed segments; extensions inject the extended type's path segments). The mangler splits on `.` and encodes each segment as a length-prefixed ident.

```
"main"                   → 4_main
"std.collections.Array"  → N3_std11_collections5_ArrayE
```

Overloads are disambiguated by the signature, return type, and instantiation — not by the path.

If a symbol surprises you, check the exact entity name in `MonoModule.entity_names` (fed from `MirModule.entity_names`) before concluding the mangler is wrong.

### Closures

A closure's path is its parent function's path with `closure` and a zero-based index appended (`lib/kestrel-mir-lower/src/body/closure.rs`):

```
"Main.foo.closure.0"
  → N 4_Main 3_foo 7_closure 1_0 E
```

The trailing `1_0` is a length-1 ident whose byte is `'0'` — not the integer 10. The `_` separator makes this unambiguous.

## Receiver convention

After the path, methods emit a single-byte receiver marker (from `ParamConvention`):

| Marker | Convention |
|--------|------------|
| `r` | `Borrow` (shared borrow) |
| `m` | `MutBorrow` (mutating borrow) |
| `c` | `Consuming` (`Self` by value) |

Free functions, initializers, and statics emit nothing here. Without this marker, a borrowing getter and a mutating setter with the same name would produce identical symbols.

## Signature

The signature is `Z` ... `E`, **omitted entirely when the parameter list is empty** (there is no `ZE` for zero params). Each parameter is:

1. `L` ident — only if the parameter has an external label (internal names are invisible).
2. `r` or `m` — only for `Borrow` / `MutBorrow` convention parameters; `Consuming` params have no marker.
3. The parameter's type.

The receiver is **not** in the signature — it is passed to `mangle_function` separately and encoded by the receiver marker above.

Examples:

```
()                                → (no signature emitted)
(_ a: Int64, _ b: Int64)          → Z i8 i8 E
(x: Int64, y: Int64)              → Z L1_x i8 L1_y i8 E
(at value: Int64)                 → Z L2_at i8 E
(_ a: Int64, _ b: Int64)          → Z i8 ri8 E     -- when b is a Borrow-convention param
```

## Return type

After the signature (or directly after the path/receiver when the signature is omitted), the return type is emitted as `R` type. It disambiguates overloads that differ only in return type. The grammar writes it as optional (`return?`), but the current implementation always emits it — even for `Unit`:

```
main() -> Unit   →   _K04_main R TvE   =   _K04_mainRTvE
```

## Instantiation

After the return type, non-empty generic type arguments are `I` ... `E`:

```
Array.append  instantiated at Int64, borrow receiver
  →  _K0N5_Array6_appendErRTvEIi8E
```

If there are no type arguments, nothing is emitted.

## Protocol extension Self disambiguation

When a protocol extension method is monomorphized for a specific conforming type, the concrete Self type is appended as a suffix `S_` type so the same method emitted for different conformers gets distinct symbols:

```
Iterator.next (for Self = ArrayIterator[Int64])
  →  _K0 N8_Iterator4_nextE R TvE S_ 13_ArrayIteratorIi8E
  =  "_K0N8_Iterator4_nextERTvES_13_ArrayIteratorIi8E"
```

The `S_` marker is unambiguous in its position because no type encoding starts with `S`.

## Type encoding

Types form their own LL(1) sublanguage (`mangle_type`). Every type production starts with a character that uniquely identifies it.

### Primitives

| `MirTy` | Encoding |
|------|----------|
| `Bool` | `b` |
| `Str` | `s` |
| `Never` | `n` |
| `I8` (Int8 / UInt8) | `i1` |
| `I16` (Int16 / UInt16) | `i2` |
| `I32` (Int32 / UInt32) | `i4` |
| `I64` (Int64 / UInt64) | `i8` |
| `F16` | `f2` |
| `F32` | `f4` |
| `F64` | `f8` |

Integer size is in bytes, not bits — `i8` is Int64, **not** Int8. There is no primitive for `Unit`: the unit type is the empty tuple (see below).

### Pointers and references

| Kestrel | Encoding |
|---------|----------|
| `Pointer[T]` | `P` type |
| `&T` (`MirTy::Ref { mutating: false }`) | `R` type |
| `&mutating T` (`MirTy::Ref { mutating: true }`) | `Rm` type |

```
&Int64           →  Ri8
&mutating Int64  →  Rmi8
```

`R`/`Rm` must stay distinct from both `P` and the bare pointee mangle — `-> &T` and `-> T` are different ABIs. (The `m` cannot be confused with a pointee type because no type production starts with `m`.)

### Tuples

```
T type* E
```

The empty tuple (`Unit`) gets a `v` marker byte so it isn't the bare pair `TE`:

```
()             →  TvE
(Int32, Bool)  →  Ti4bE
```

### Function types

```
thin:  F count "_" type{count} ret E
thick: C count "_" type{count} ret E
```

The count is the number of parameters (decimal). A `_` separator makes the digit run unambiguous with the type bytes that follow. Parameter conventions are **not** encoded inside function types.

```
func(Int32, Int32) -> Bool           →  F2_i4i4bE
func escaping(Int64) -> Unit         →  C1_i8TvEE
```

### Named types

```
path ("I" type+ "E")?
```

A named type encodes as its path (either a single ident or `N...E`), optionally followed by its type arguments. The mangler resolves the entity's name from the `entity_names` map.

```
Array              →  5_Array
Array[Int64]       →  5_ArrayIi8E
std.Array[Int64]   →  N3_std5_ArrayEIi8E
```

### Type parameters and associated projections

`MirTy::TypeParam` and `MirTy::AssociatedProjection` have **no encoding** — mangling runs after monomorphization, so if either reaches `mangle_type` it panics with "monomorphization bug". (Concrete Self is handled at symbol level by the `S_` suffix, not in type position.)

### Error

Types that failed to lower become `MirTy::Error` and encode as `X`. This lets the pipeline keep building symbols without aborting when upstream analysis emitted a diagnostic.

## LL(1) parse table

**Symbol entry (after `_K0`):**

| First char | Production |
|------------|-----------|
| `N` | nested path — read idents until `E` |
| `[1-9]` | simple path — read one ident |

**After path:**

| First char | Production |
|------------|-----------|
| `r` / `m` / `c` | receiver marker |
| `Z` | signature |
| `R` | return type — recurse one type |
| `I` | instantiation |
| `S` followed by `_` | self-disambig suffix |
| end | done |

Position resolves the `R`-return vs `R`-ref and `r`/`m`-receiver vs `r`/`m`-param-convention overlaps: the return marker appears exactly once after the (optional) signature, and convention markers only appear inside a signature directly before a type.

**Type position:**

| First char | Production |
|------------|-----------|
| `b` / `s` / `n` | primitive |
| `i` | integer — read one size byte |
| `f` | float — read one size byte |
| `P` | pointer — recurse one type |
| `R` | reference — optional `m` (mutating), then recurse one type |
| `T` | tuple — read types until `E` (`v` marks the empty tuple) |
| `F` / `C` | function — read count, `_`, params, return, `E` |
| `X` | Error type |
| `N` | nested named type — read idents until `E`, optional `I...E` |
| `[1-9]` | simple named type — read ident, optional `I...E` |

No two productions share a first character. There is no backtracking.

## Worked examples

All drawn from the unit tests in `lib/kestrel-mir/src/mono/mangle.rs`.

| Symbol | Mangled |
|--------|---------|
| `main() -> Unit` | `_K04_mainRTvE` |
| `add(_ x: Int64, _ y: Int64) -> Unit` (`y` by borrow) | `_K03_addZi8ri8ERTvE` |
| `std.Array.count` (borrow receiver, no params) | `_K0N3_std5_Array5_countErRTvE` |
| `Array.append` at `[Int64]` (borrow receiver) | `_K0N5_Array6_appendErRTvEIi8E` |
| `Iterator.next` for Self = `ArrayIterator[Int64]` | `_K0N8_Iterator4_nextERTvES_13_ArrayIteratorIi8E` |

## Demangling

The grammar above is enough to write a recursive-descent demangler. Pseudocode:

```
demangle(s):
    expect "_K0"
    path = read_path()
    recv = read_opt("rmc")
    sig  = peek == "Z" ? read_sig() : ""
    ret  = (expect "R"; read_type())
    inst = peek == "I" ? read_inst() : ""
    selfsuf = peek == "S" && peek_next == "_" ? read_self_suffix() : ""
    return format(path, recv, sig, ret, inst, selfsuf)

read_path():
    if peek == 'N': consume; segs = [read_ident until 'E']; consume 'E'
    else:            segs = [read_ident()]
    return segs joined with "."

read_ident():
    len = read_decimal()
    consume '_'
    return consume_bytes(len)

read_sig():
    consume 'Z'; params = []
    while peek != 'E':
        label = (peek == 'L') ? (consume 'L'; read_ident()) : None
        conv  = read_opt("rm")
        ty    = read_type()
        params.push((label, conv, ty))
    consume 'E'
    return params

read_type(): match on peek per the LL(1) table above
```

## Adding a new encoding

If you add a new `MirTy` variant or a new function kind, do all of:

1. Pick a first character that isn't already claimed in type position (see the LL(1) table).
2. Add the encoding branch to `mangle_type` in `lib/kestrel-mir/src/mono/mangle.rs`.
3. Add a unit test showing the expected output.
4. Update this document's LL(1) table and the "Type encoding" section.
5. If the new variant can appear at symbol level (not just inside types), update the top-level grammar too.

Two encodings on the same first character means ambiguity — if you can't find a free letter, introduce a two-byte marker (like `S_` for self-disambig) rather than reusing one.

## MirTy coverage checklist

| Variant | Encoding | Section |
|---------|----------|---------|
| `I8` / `I16` / `I32` / `I64` | `i1` / `i2` / `i4` / `i8` | Primitives |
| `F16` / `F32` / `F64` | `f2` / `f4` / `f8` | Primitives |
| `Bool` / `Never` / `Str` | `b` / `n` / `s` | Primitives |
| `Tuple(empty)` | `TvE` | Tuples |
| `Tuple(non-empty)` | `T...E` | Tuples |
| `Pointer(inner)` | `P` type | Pointers and references |
| `Ref { mutating: false }` | `R` type | Pointers and references |
| `Ref { mutating: true }` | `Rm` type | Pointers and references |
| `Named { entity, type_args }` | path (`I...E`)? | Named types |
| `FuncThin` / `FuncThick` | `F count _ ...E` / `C count _ ...E` | Function types |
| `TypeParam(entity)` | **panics** — mono bug | Type parameters |
| `AssociatedProjection` | **panics** — mono bug | Type parameters |
| `Error` | `X` | Error |
