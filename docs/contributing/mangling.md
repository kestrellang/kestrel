# Kestrel Symbol Mangling — v0 Scheme

Every Kestrel function, method, initializer, deinit, static, and closure needs a unique linker symbol. The v0 mangler encodes source-level names, module paths, parameter labels, receiver conventions, and generic instantiations into a flat string that:

1. Is unique for every distinct monomorphized symbol.
2. Is deterministic — same input always produces the same output.
3. Is LL(1) parseable, so it can be demangled without backtracking.
4. Uses only characters legal in linker symbols (alphanumeric + `_`).

The implementation lives in `lib/kestrel-codegen/src/mangle.rs`. The public entry points are `mangle_name`, `mangle_function`, and `mangle_function_with_self`.

## When mangling is bypassed

| Symbol | Linker name | Reason |
|--------|-------------|--------|
| Program entry point | `main` | C runtime expects it. |
| `extern` functions | declared name verbatim | FFI. |
| Static initializer | `__kestrel_init_statics` | Fixed ABI. |

Everything else goes through the mangler.

## Top-level grammar

```
mangled       = "_K0" path receiver? signature? instantiation? self-disambig?
path          = ident                          -- single unqualified name
              | "N" ident+ "E"                 -- nested (qualified) name
ident         = length "_" utf8-bytes          -- byte-length-prefixed
length        = [1-9] [0-9]*                   -- decimal, no leading zeros
receiver      = "r" | "m" | "c"                -- Ref / RefMut / consuming
signature     = "Z" param* "E"
param         = ("L" ident)? type              -- optional external label, then type
instantiation = "I" type+ "E"                  -- generic type args
self-disambig = "S_" type                      -- concrete Self for protocol ext methods
```

The `_` between `length` and bytes makes identifiers that start with a digit (closure indices, for example) unambiguous. The `E` terminator on `N`, `Z`, `I`, `T`, `F`, `C` makes every composite production self-delimiting.

All mangled symbols start with `_K0` — `K` identifies Kestrel, `0` is the version. Future revisions bump the digit.

## Path encoding

Entity names in MIR are dot-separated (`"std.collections.Array.append"`). The mangler splits on `.` and encodes each segment as a length-prefixed ident.

```
"main"                   → 4_main
"std.collections.Array"  → N3_std11_collections5_ArrayE
```

The last path segment has any `$` disambiguation suffix stripped before encoding. Overloads are disambiguated in the signature, not the path.

```
"std.foo$1"  → N3_std3_fooE
```

### Getters, setters, subscripts

Accessor paths carry their kind as part of the ident — the colon and `$` are part of the bytes, counted in the length prefix. For example, a getter for `Point.x` has the MIR name `"Main.Point.get:x"`, which mangles to:

```
N 4_Main 5_Point 5_get:x E
```

Subscript getters include labels separated by `$`, and **those `$` symbols are part of the final segment's bytes** (the stripping rule only removes a `$suffix` the overload disambiguator adds — the subscript's own `$` labels survive because the segment starts with `get:subscript` or similar, and the stripping splits on the first `$`, which is the label separator). In practice, subscripts produce the complete name in MIR and the mangler stores it byte-for-byte after the first segment.

If you see a surprise here, check the exact entity name in `MirModule.entity_names` before concluding the mangler is wrong.

### Closures

A closure's path is its parent function's path with `closure` and a zero-based index appended as idents:

```
"Main.foo.closure.0"
  → N 4_Main 3_foo 7_closure 1_0 E
```

The trailing `1_0` is a length-1 ident whose byte is `'0'` — not the integer 10. The `_` separator makes this unambiguous.

## Receiver convention

After the path, methods and deinits emit a single-byte receiver marker:

| Marker | Convention |
|--------|------------|
| `r` | `Ref[Self]` |
| `m` | `RefMut[Self]` |
| `c` | consuming (`Self` by value) |

Free functions, initializers, and statics emit nothing here. Without this marker, a `Ref` getter and a `RefMut` setter with the same name would produce identical symbols.

## Signature

The signature is `Z` ... `E`. Each parameter is either:

- `L` ident type — externally-labeled parameter (the ident is the external label), or
- type alone — unlabeled parameter (declared `_ x: T`, or with no external label at all).

Rules:

1. **The receiver is not in the signature.** It's encoded separately.
2. The mangler skips the first parameter for methods and deinits; for initializers, it skips the first param iff its internal name is `self`.
3. Internal parameter names are invisible to the mangler — only external labels matter.
4. An empty signature is `ZE`.

Examples:

```
()                             → Z E
(_ a: Int64, _ b: Int64)       → Z i8 i8 E
(x: Int64, y: Int64)           → Z L1_x i8 L1_y i8 E
(at value: Int64)              → Z L2_at i8 E
(from: Int32)                  → Z L4_from i4 E
```

## Instantiation

After the signature, non-empty generic type arguments are `I` ... `E`:

```
identity[Int64](_ x: Int64)    signature: Zi8E
                              instantiation: Ii8E
```

If there are no type arguments, nothing is emitted.

## Protocol extension Self disambiguation

When a protocol extension method is monomorphized for a specific conforming type, the concrete Self type is appended as a suffix `S_` type so the same method emitted for different conformers gets distinct symbols:

```
std.Iterator.next (for Self = ArrayIterator[Int64], RefMut receiver)
  →  _K0 N3_std8_Iterator4_nextE m Z E S_ 13_ArrayIteratorIi8E
  =  "_K0N3_std8_Iterator4_nextEmZES_13_ArrayIteratorIi8E"
```

The `S_` prefix is reserved — bare `S` means "unresolved `Self`" as a type, and `S_` unambiguously starts the disambiguator because a type encoding never starts with `_`.

## Type encoding

Types form their own LL(1) sublanguage. Every type production starts with a character that uniquely identifies it.

### Primitives

| Type | Encoding |
|------|----------|
| `Bool` | `b` |
| `Str` | `s` |
| `Unit` (the empty tuple) | `v` |
| `Never` | `n` |
| `Int8` / `UInt8` | `i1` |
| `Int16` / `UInt16` | `i2` |
| `Int32` / `UInt32` | `i4` |
| `Int64` / `UInt64` | `i8` |
| `Float16` | `f2` |
| `Float32` | `f4` |
| `Float64` | `f8` |

Integer size is in bytes, not bits — `i8` is Int64, **not** Int8.

### References and pointers

| Kestrel | Encoding |
|---------|----------|
| `Pointer[T]` | `P` type |
| `Ref[T]` | `R` type |
| `RefMut[T]` | `M` type |

```
Ref[Int64]  →  Ri8
```

### Tuples

```
T type* E
```

`()` is special — it encodes as `v` to match `Unit`. Non-empty tuples use `T...E`:

```
(Int32, Bool)  →  Ti4bE
```

### Function types

```
thin:  F count "_" type{count} ret E
thick: C count "_" type{count} ret E
```

The count is the number of parameters (decimal). A `_` separator makes the digit run unambiguous with the type bytes that follow.

```
func(Int32, Int32) -> Bool           →  F2_i4i4bE
func escaping(Int64) -> Unit         →  C1_i8vE
```

### Named types

```
path ("I" type+ "E")?
```

A named type encodes as its path (either a single ident or `N...E`), optionally followed by its type arguments. The mangler resolves the underlying entity's name from `MirModule.entity_names` via `entity`.

```
Array              →  5_Array
Array[Int64]       →  5_ArrayIi8E
std.Int64          →  N3_std5_Int64E
```

### Type parameters

Inside a generic body, a `MirTy::TypeParam(entity)` encodes as a bare ident — the entity's name:

```
T  →  1_T
```

Concrete codegen monomorphizes these away; they only appear in un-monomorphized MIR.

### Self

| Form | Encoding |
|------|----------|
| `SelfType` with no concrete substitution | `S` |
| `SelfType` with `self_type` set on the mangler | the encoded concrete type |

The mangler clears `self_type` before recursing into the substituted type to avoid infinite recursion when the concrete type itself refers to Self (common in iterator chains).

### Associated type projection

Associated type projections include the protocol name to prevent collisions between different protocols that define the same associated-type name (e.g. `Iterator.Item` vs `Container.Item` on the same base type):

```
Q base p protocol-path assoc-ident
```

```
Self.Iterator.Item  →  QSp8_Iterator4_Item
```

The `p` marker is a single byte. It's lowercase, keeping it distinct from the type-production starters above.

### Error

Types that failed to lower become `MirTy::Error` and encode as `X`. This lets codegen keep building symbols without aborting when upstream analysis emitted a diagnostic.

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
| `I` | instantiation |
| `S` followed by `_` | self-disambig suffix |
| end | done |

**After receiver:** same as after path, minus receiver.

**Type position:**

| First char | Production |
|------------|-----------|
| `b` / `s` / `v` / `n` | primitive |
| `i` | integer — read one size byte |
| `f` | float — read one size byte |
| `P` / `R` / `M` | pointer / ref / refmut — recurse one type |
| `T` | tuple — read types until `E` |
| `F` / `C` | function — read count, `_`, params, return, `E` |
| `S` | unresolved Self |
| `Q` | associated projection — read base, `p`, protocol path, assoc ident |
| `X` | Error type |
| `N` | nested named type — read idents until `E`, optional `I...E` |
| `[1-9]` | simple named type — read ident, optional `I...E` |

No two productions share a first character. There is no backtracking.

## Worked examples

Drawn from `lib/kestrel-codegen/src/mangle.rs` tests.

### Functions

| Source | Mangled |
|--------|---------|
| `main()` | `_K04_main` |
| `add(x: Int64, y: Int64) -> Int64` | `_K03_addZi8i8E` |
| `insert(at value: Int64)` | `_K06_insertZL2_ati8E` |
| `identity[Int64]()` | `_K08_identityZEIi8E` |

### Named paths

| Name in MIR | Mangled |
|-------------|---------|
| `std.collections.Array` | `_K0N3_std11_collections5_ArrayE` |
| `std.foo$1` | `_K0N3_std3_fooE` |
| `Array[Int64]` (as name) | `_K05_ArrayIi8E` |

### Methods

```
std.Array.count()   receiver = Ref
  →  _K0 N3_std5_Array5_countE r Z E
  =  _K0N3_std5_Array5_countErZE
```

### Initializers

```
Point.init(x: Int64, y: Int64)
  →  _K0 N5_Point4_initE Z i8 i8 E
  =  _K0N5_Point4_initEZi8i8E
```

Self param is skipped because the first param is named `self`. Labels `x`/`y` are unlabeled in this example because they were declared without external labels; if the source said `init(x x: Int64, y y: Int64)` you'd get `ZL1_xi8L1_yi8E`.

### Protocol extension with Self disambiguation

```
std.Iterator.next   receiver = RefMut   Self = ArrayIterator[Int64]
  →  _K0 N3_std8_Iterator4_nextE m Z E S_ 13_ArrayIteratorIi8E
  =  _K0N3_std8_Iterator4_nextEmZES_13_ArrayIteratorIi8E
```

### Associated projection inside a signature

```
func foo(_ x: Self.Iterator.Item) -> Unit
  signature:  Z QSp8_Iterator4_Item E
```

## Demangling

The grammar above is enough to write a recursive-descent demangler. Pseudocode:

```
demangle(s):
    expect "_K0"
    path = read_path()
    recv = read_opt("rmc")
    sig  = peek == "Z" ? read_sig() : ""
    inst = peek == "I" ? read_inst() : ""
    selfsuf = peek == "S" && peek_next == "_" ? read_self_suffix() : ""
    return format(path, recv, sig, inst, selfsuf)

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
        ty    = read_type()
        params.push((label, ty))
    consume 'E'
    return params

read_type(): match on peek per the LL(1) table above
```

## Adding a new encoding

If you add a new `MirTy` variant or a new function kind, do all of:

1. Pick a first character that isn't already claimed in type position (see the LL(1) table).
2. Add the encoding branch to `Mangler::mangle_type` in `lib/kestrel-codegen/src/mangle.rs`.
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
| `Tuple(empty)` | `v` | Tuples |
| `Tuple(non-empty)` | `T...E` | Tuples |
| `Pointer(T)` / `Ref(T)` / `RefMut(T)` | `P` / `R` / `M` | References |
| `Named { entity, type_args }` | path (`I...E`)? | Named types |
| `TypeParam(entity)` | bare ident | Type parameters |
| `FuncThin` / `FuncThick` | `F count _ ...E` / `C count _ ...E` | Function types |
| `SelfType` | `S` (abstract) or substituted | Self |
| `AssociatedProjection` | `Q base p protocol ident` | Associated projection |
| `Error` | `X` | Error |
