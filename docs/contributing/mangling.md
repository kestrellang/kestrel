# Kestrel Symbol Mangling Specification v0

## Introduction

Every Kestrel function, method, initializer, static, and closure needs a unique linker symbol. The mangling scheme encodes source-level names, module paths, parameter labels, and generic instantiations into a flat string that:

1. Is unique for every distinct symbol
2. Is deterministic (same source always produces the same symbol)
3. Can be demangled back to a human-readable form
4. Uses only characters valid in linker symbols (alphanumeric + `_`)
5. Is LL(1) parseable — every production is determined by its first character

### Prefix

All mangled symbols begin with `_K0` — the `K` identifies Kestrel, the `0` is the version. Future revisions to the scheme increment this digit.

### When mangling is bypassed

- `main` — the program entry point uses the bare name `main`
- `extern` functions — use their declared name verbatim
- `__kestrel_init_statics` — the static initializer uses a fixed name

## Grammar

```
symbol     ::= '_K0' path recv? sig? inst?

path       ::= ident                    -- single unqualified name
             | 'N' ident+ 'E'           -- nested (qualified) name

ident      ::= length '_' utf8-bytes    -- length-prefixed identifier with '_' separator
length     ::= [1-9] [0-9]*             -- decimal integer, no leading zeros

recv       ::= 'r'                      -- Ref[Self] receiver
             | 'm'                      -- RefMut[Self] receiver
             | 'c'                      -- consuming (Self by value) receiver

sig        ::= 'Z' param* 'E'          -- parameter signature
param      ::= type                     -- unlabeled parameter (no external label)
             | 'L' ident type           -- labeled parameter

inst       ::= 'I' type+ 'E'           -- generic instantiation

type       ::= primitive
             | 'P' type                 -- Pointer
             | 'R' type                 -- Ref
             | 'M' type                 -- RefMut
             | 'T' type* 'E'           -- Tuple
             | 'F' count '_' type{count} type 'E'  -- thin function
             | 'C' count '_' type{count} type 'E'  -- thick function (closure)
             | 'S'                      -- Self type
             | 'Q' type ident           -- associated type projection
             | 'X'                      -- Error type
             | named-type               -- user-defined type

primitive  ::= 'b'                      -- Bool
             | 's'                      -- Str
             | 'v'                      -- Unit (void)
             | 'n'                      -- Never
             | 'i' size                 -- integer
             | 'f' size                 -- float

size       ::= '1' | '2' | '4' | '8'   -- byte width

count      ::= [0-9]+                   -- decimal integer (for function type arity)

named-type ::= path ('I' type+ 'E')?   -- optionally generic
```

The `_` separator in `ident` makes numeric identifiers unambiguous. For example, closure index `0` is encoded as `1_0` (length 1, separator, byte `0`), not `10` which could be misread as length 10.

The `count` in function types (`F`/`C`) is also followed by `_`, making it consistent with `ident` and unambiguous regardless of digit count.

### LL(1) parse table for `type`

| First char | Production | Notes |
|------------|------------|-------|
| `b` | `Bool` | |
| `s` | `Str` | |
| `v` | `Unit` | |
| `n` | `Never` | |
| `i` | integer — read size | `i1`=I8, `i2`=I16, `i4`=I32, `i8`=I64 |
| `f` | float — read size | `f2`=F16, `f4`=F32, `f8`=F64 |
| `P` | `Pointer(type)` | |
| `R` | `Ref(type)` | |
| `M` | `RefMut(type)` | |
| `T` | `Tuple` — read types until `E` | |
| `F` | thin function — read count, `_`, params, ret, `E` | |
| `C` | thick function — read count, `_`, params, ret, `E` | |
| `S` | `SelfType` | |
| `Q` | associated type projection — read base type, then ident | |
| `X` | `Error` | |
| `N` | nested named type — read idents until `E`, optional `I...E` | |
| `[1-9]` | simple named type — read ident, optional `I...E` | |

**No two productions share a first character.** The parser never backtracks.

### LL(1) parse table for `symbol` (after `_K0`)

| First char | Next production |
|------------|-----------------|
| `N` | nested path — read idents until `E` |
| `[1-9]` | simple path — read single ident |

After the path:

| First char | Next production |
|------------|-----------------|
| `r` | ref receiver |
| `m` | refmut receiver |
| `c` | consuming receiver |
| `Z` | signature — read params until `E` |
| `I` | instantiation (no signature) |
| end-of-string | done |

After the optional receiver:

| First char | Next production |
|------------|-----------------|
| `Z` | signature — read params until `E` |
| `I` | instantiation (no signature) |
| end-of-string | done |

The receiver markers `r`, `m`, `c` are unambiguous at this position — they cannot conflict with `Z`, `I`, or end-of-string. Lowercase letters are reserved for primitives in type position, but this is not type position.

## Path encoding

The path encodes the fully qualified name of the symbol: module, type, and member.

### Simple (unqualified) names

A bare function at the top level:

```
path = ident
```

Example: a function `foo` → `3_foo`

### Nested (qualified) names

Most symbols are nested inside a module and possibly a type:

```
path = 'N' ident+ 'E'
```

Example: `Main.Point.init` → `N4_Main5_Point4_initE`

### Closures

Closures are named by appending `closure` and a zero-based index to their parent function's path:

```
Main.foo.closure.0 → N 4_Main 3_foo 7_closure 1_0 E
```

The index `0` is encoded as the ident `1_0` (length 1, separator, character `0`).

### Getters and setters

Property accessors use `get:` and `set:` prefixes as part of the ident:

```
Main.Point.get:x → N 4_Main 5_Point 5_get:x E
Main.Point.set:x → N 4_Main 5_Point 5_set:x E
```

The colon is part of the identifier bytes. The length counts all bytes including the colon.

### Subscripts

Subscript accessors include parameter labels separated by `$`:

```
Main.Grid.get:subscript$row$col → N 4_Main 4_Grid 20_get:subscript$row$col E
```

### Operators

Operators are encoded by their semantic name as an identifier in the path:

```
Main.Int64.add       → N 4_Main 5_Int64 3_add E
Main.Int64.equals    → N 4_Main 5_Int64 6_equals E
Main.Int64.negate    → N 4_Main 5_Int64 6_negate E
```

### Deinit

Deinitializers use `deinit` as their path segment:

```
Main.Resource.deinit → N 4_Main 8_Resource 6_deinit E
```

### Extensions and protocol extension methods

For extension methods (including protocol extensions), the **implementing type** appears in the path, not the protocol or extension:

```
// Protocol Equatable defines notEquals.
// Int64 conforms to Equatable via an extension.
// The mangled path uses Int64, not Equatable:
Main.Int64.notEquals → N 4_Main 5_Int64 9_notEquals E
```

This means two types conforming to the same protocol produce different symbols, which is correct since each conformance generates a distinct monomorphized function.

## Receiver convention

Methods have a receiver (`self`) that is passed implicitly. The receiver convention — whether `self` is passed by `Ref`, `RefMut`, or by value (consuming) — is encoded as an optional marker between the path and the signature:

```
recv = 'r'    -- Ref[Self] receiver
     | 'm'    -- RefMut[Self] receiver
     | 'c'    -- consuming (Self by value) receiver
```

Free functions, initializers, and statics have no receiver marker. Without the marker, two methods that differ only in receiver convention (e.g., a `Ref` getter and a `RefMut` getter) would collide.

### Examples

```
Main.Point.get:x()              → _K0 N...E r Z E        -- getter takes Ref
Main.Point.set:x(Int64)         → _K0 N...E m Z i8 E     -- setter takes RefMut
Main.Resource.deinit()          → _K0 N...E c Z E         -- deinit consumes
Main.add(_ a: Int64, _ b: Int64) → _K0 N...E Z i8 i8 E   -- free function, no recv
Main.Point.init(x: Int64, ...)  → _K0 N...E Z L1_x i8... -- init, no recv
```

## Signature encoding

The signature section encodes parameter labels and types. It is delimited by `Z...E`.

```
sig = 'Z' param* 'E'
```

### Rules

1. **Self is excluded** — the receiver is not part of the signature (it's in the path); the receiver convention is encoded separately
2. **Labeled parameter**: `L` + ident (the external label) + type
3. **Unlabeled parameter** (no external label): bare type only
4. **Position matters** — parameters are encoded in declaration order
5. **Internal names don't matter** — only external labels affect the mangling
6. **Default is unlabeled** — if a parameter has no explicit external label, it is treated as unlabeled (bare type, no `L` prefix)

### Examples

```
(x: Int64, y: Int64)       → Z L1_x i8 L1_y i8 E
(_ a: Int64, _ b: Int64)   → Z i8 i8 E
(a: Int64, b: Int64)       → Z i8 i8 E               -- no explicit label = unlabeled
(_ a: Int8, x: Int8)       → Z i1 L1_x i1 E
(x: Int8, _ a: Int8)       → Z L1_x i1 i1 E
()                          → Z E
(from: Int32)               → Z L4_from i4 E
(dx: Int64, dy: Int64)     → Z L2_dx i8 L2_dy i8 E
```

Two functions that differ only in internal parameter names (e.g., `(_ a: Int64)` vs `(_ b: Int64)`) produce identical signatures. This is correct — they have the same external API and are not valid overloads of each other.

## Instantiation

After monomorphization, each generic function is instantiated with concrete types. The instantiation section records these type arguments:

```
inst = 'I' type+ 'E'
```

Example: `identity[Int64]` appends `Ii8E`.

The instantiation section appears after the signature (if present):

```
_K0 path recv? Z params E I types E
```

## Type encoding

### Primitives

| Kestrel type | Encoding | MirTy variant |
|--------------|----------|---------------|
| `Bool` | `b` | `Bool` |
| `String` | `s` | `Str` |
| `Unit` | `v` | `Unit` |
| `Never` | `n` | `Never` |
| `Int8` / `UInt8` | `i1` | `I8` |
| `Int16` / `UInt16` | `i2` | `I16` |
| `Int32` / `UInt32` | `i4` | `I32` |
| `Int64` / `UInt64` | `i8` | `I64` |
| `Float16` | `f2` | `F16` |
| `Float32` | `f4` | `F32` |
| `Float64` | `f8` | `F64` |

The size suffix is the byte width: 1, 2, 4, or 8.

### Pointers and references

| Kestrel type | Encoding |
|-------------|----------|
| `Pointer[T]` | `P` type |
| `Ref[T]` | `R` type |
| `RefMut[T]` | `M` type |

Example: `Ref[Int64]` → `Ri8`

### Tuples

```
'T' type* 'E'
```

Example: `(Int64, Bool)` → `Ti8bE`, `()` → `TE`

### Function types

```
thin:  'F' count '_' type{count} type 'E'
thick: 'C' count '_' type{count} type 'E'
```

The count is the number of parameters (decimal). After the `_` separator come exactly `count` parameter types, then the return type, then `E`.

Example: `func(Int64) -> Bool` → `F1_i8bE`, `func escaping(Int64, Int64) -> Int64` → `C2_i8i8i8E`

### Named types

Named types use the same path encoding as symbols:

```
named-type ::= path ('I' type+ 'E')?
```

- Simple: `5_Point`
- Qualified: `N3_std5_ArrayE`
- Generic: `N3_std5_ArrayEIi8E` (Array[Int64])

**Note on greedy parsing:** Inside a `Z...E` signature or another type context, `I` after a named type is greedily attached as that type's generic arguments. Only an `I` appearing after the signature's closing `E` is parsed as the symbol-level instantiation.

### Special types

| Type | Encoding | Notes |
|------|----------|-------|
| `Self` | `S` | Only in unmonomorphized protocol signatures |
| `T.Element` | `Q` type ident | Associated type projection |
| `Error` | `X` | Poison type from failed lowering |

### LL(1) ambiguity analysis

The critical check: can `S` (SelfType) be confused with a named type starting with `S`? No — named types start with a digit (`[1-9]`) or `N`. Single-letter codes are reserved for built-in types.

Can `i` (integer prefix) be confused with a named type? No — named types always start with a length digit. The letter `i` is not a digit.

## Statics

Static variables (module-level fields and `static` struct fields) are mangled like any other nested path, but with no signature:

```
Main.counter → _K0 N 4_Main 7_counter E
```

**Note**: the current implementation has a bug where statics use `format!("{}", name)` instead of proper mangling. The v0 scheme fixes this.

## Comprehensive examples

### Functions

| Source | Mangled |
|--------|---------|
| `Main.add(_ a: Int64, _ b: Int64)` | `_K0N4_Main3_addEZi8i8E` |
| `Main.greet(name: String)` | `_K0N4_Main5_greetEZL4_namesE` |

### Initializers

| Source | Mangled |
|--------|---------|
| `Main.Point.init(x: Int64, y: Int64)` | `_K0N4_Main5_Point4_initEZL1_xi8L1_yi8E` |
| `Main.Point.translate(dx: Int64, dy: Int64)` | `_K0N4_Main5_Point9_translateEmZL2_dxi8L2_dyi8E` |
| `std.Int64.init(from: Int32)` | `_K0N3_std5_Int644_initEZL4_fromi4E` |
| `std.Int64.init(from: Int8)` | `_K0N3_std5_Int644_initEZL4_fromi1E` |

### Generics

| Source | Mangled |
|--------|---------|
| `Main.identity[Int64](_ x: Int64)` | `_K0N4_Main8_identityEZi8EIi8E` |
| `Main.Box[Int64].unwrap()` | `_K0N4_Main3_Box6_unwrapErZEIi8E` |
| `std.Array[Int64].append(_ element: Int64)` | `_K0N3_std5_Array6_appendEmZi8EIi8E` |

### Operators and protocol extensions

| Source | Mangled |
|--------|---------|
| `Main.Int64.add(_ other: Ref[Int64])` | `_K0N4_Main5_Int643_addErZRi8E` |
| `Main.Int64.notEquals(_ other: Ref[Int64])` | `_K0N4_Main5_Int649_notEqualsErZRi8E` |

### Getters and setters

| Source | Mangled |
|--------|---------|
| `Main.Point.get:x()` | `_K0N4_Main5_Point5_get:xErZE` |
| `Main.Point.set:x(Int64)` | `_K0N4_Main5_Point5_set:xEmZi8E` |

### Statics

| Source | Mangled |
|--------|---------|
| `Main.counter` | `_K0N4_Main7_counterE` |

### Closures

| Source | Mangled |
|--------|---------|
| `Main.foo.closure.0()` | `_K0N4_Main3_foo7_closure1_0EZE` |

### Deinit

| Source | Mangled |
|--------|---------|
| `Main.Resource.deinit()` | `_K0N4_Main8_Resource6_deinitEcZE` |

## Demangling

The following pseudocode implements a recursive-descent demangler for v0 symbols.

```
function demangle(input):
    expect "_K0"
    path = read_path()
    recv = ""
    sig  = ""
    inst = ""
    if peek() in ['r', 'm', 'c']:
        recv = read_recv()
    if peek() == 'Z':
        sig = read_sig()
    if peek() == 'I':
        inst = read_inst()
    return format("{path}{inst}({recv}{sig})")

function read_path():
    if peek() == 'N':
        consume 'N'
        segments = []
        while peek() != 'E':
            segments.append(read_ident())
        consume 'E'
        return segments.join(".")
    else:
        return read_ident()

function read_ident():
    len = read_decimal()
    consume '_'
    return consume_bytes(len)

function read_recv():
    c = consume
    if c == 'r': return "ref "
    if c == 'm': return "refmut "
    if c == 'c': return "consuming "

function read_sig():
    consume 'Z'
    params = []
    while peek() != 'E':
        if peek() == 'L':
            consume 'L'
            label = read_ident()
            ty = read_type()
            params.append("{label}: {ty}")
        else:
            ty = read_type()
            params.append("_ : {ty}")
    consume 'E'
    return params.join(", ")

function read_inst():
    consume 'I'
    types = []
    while peek() != 'E':
        types.append(read_type())
    consume 'E'
    return "[" + types.join(", ") + "]"

function read_type():
    c = peek()
    if c == 'b': consume; return "Bool"
    if c == 's': consume; return "Str"
    if c == 'v': consume; return "Unit"
    if c == 'n': consume; return "Never"
    if c == 'i': consume; sz = consume; return "Int" + size_to_bits(sz)
    if c == 'f': consume; sz = consume; return "Float" + size_to_bits(sz)
    if c == 'P': consume; return "Pointer[" + read_type() + "]"
    if c == 'R': consume; return "Ref[" + read_type() + "]"
    if c == 'M': consume; return "RefMut[" + read_type() + "]"
    if c == 'T': consume; types = []; while peek() != 'E': types.append(read_type()); consume 'E'; return "(" + types.join(", ") + ")"
    if c == 'F' or c == 'C':
        kind = consume
        count = read_decimal()
        consume '_'
        params = [read_type() for _ in range(count)]
        ret = read_type()
        consume 'E'
        prefix = "func" if kind == 'F' else "func escaping"
        return prefix + "(" + params.join(", ") + ") -> " + ret
    if c == 'S': consume; return "Self"
    if c == 'Q': consume; base = read_type(); assoc = read_ident(); return base + "." + assoc
    if c == 'X': consume; return "<error>"
    if c == 'N':  -- nested named type
        path = read_path()
        if peek() == 'I': inst = read_inst(); return path + inst
        return path
    if c in '1'..'9':  -- simple named type
        name = read_ident()
        if peek() == 'I': inst = read_inst(); return name + inst
        return name

function size_to_bits(sz):
    '1' -> "8", '2' -> "16", '4' -> "32", '8' -> "64"
```

## MirTy coverage checklist

Every `MirTy` variant has a defined encoding:

| MirTy variant | Encoding | Covered in |
|---------------|----------|------------|
| `I8` | `i1` | Primitives |
| `I16` | `i2` | Primitives |
| `I32` | `i4` | Primitives |
| `I64` | `i8` | Primitives |
| `F16` | `f2` | Primitives |
| `F32` | `f4` | Primitives |
| `F64` | `f8` | Primitives |
| `Bool` | `b` | Primitives |
| `Unit` | `v` | Primitives |
| `Never` | `n` | Primitives |
| `Str` | `s` | Primitives |
| `Pointer(T)` | `P` type | Pointers and references |
| `Ref(T)` | `R` type | Pointers and references |
| `RefMut(T)` | `M` type | Pointers and references |
| `Tuple(elems)` | `T` types `E` | Tuples |
| `Named { name, type_args }` | path (`I` types `E`)? | Named types |
| `TypeParam(id)` | **panic** | Should never appear after monomorphization; indicates a compiler bug |
| `FuncThin { params, ret }` | `F` count `_` types ret `E` | Function types |
| `FuncThick { params, ret }` | `C` count `_` types ret `E` | Function types |
| `SelfType` | `S` | Special types |
| `AssociatedTypeProjection` | `Q` type ident | Special types |
| `Error` | `X` | Special types |

## Symbol kind coverage checklist

Every symbol kind has a documented path encoding:

| Symbol kind | Path encoding | Section |
|-------------|---------------|---------|
| Function | `N module name E` with labels in sig | Functions |
| Initializer | `N module Type init E` with labels in sig (no recv) | Initializers |
| Deinit | `N module Type deinit E` with recv | Deinit |
| Getter | `N module Type get:field E` with recv `r` | Getters and setters |
| Setter | `N module Type set:field E` with recv `m` | Getters and setters |
| Subscript getter | `N module Type get:subscript$labels E` with recv | Subscripts |
| Static field | `N module name E` (no sig, no recv) | Statics |
| Closure | `N module func closure index E` | Closures |
| Operator | `N module Type opname E` with recv | Operators |
| Protocol extension method | implementing type in path, with recv | Extensions |

## Changes from current implementation

The v0 scheme differs from the current `_K` mangler in several ways:

1. **Prefix**: `_K0` instead of `_K` — versioned for future evolution
2. **Nested path delimiter**: `N...E` instead of concatenated length-prefixed segments (makes path boundaries unambiguous)
3. **Ident separator**: `length '_' bytes` instead of `length bytes` — unambiguous for numeric identifiers (e.g., closure indices)
4. **Signature section**: `Z...E` with labeled params (`L` ident type) instead of `P` count types — encodes parameter labels, not just types
5. **Receiver convention**: `r`/`m`/`c` marker between path and signature — distinguishes methods by receiver type
6. **Integer/float encoding**: `i1`/`i2`/`i4`/`i8` and `f2`/`f4`/`f8` (byte widths) instead of `i8`/`i16`/`i32` (bit strings) — avoids variable-length ambiguity
7. **Unit**: `v` instead of `u` — frees `u` for potential unsigned integer types
8. **Tuple**: `T...E` delimited instead of `T` count types — simpler parsing
9. **Function types**: `F`/`C` count `_` params ret `E` — adds `_` separator and `E` terminator for unambiguous parsing
10. **Statics**: properly mangled with `_K0` prefix instead of `format!("{}", name)`
11. **No `$` encoding in path segments** — labels move to the signature section where they belong
12. **No `S_` self-type suffix** — implementing type goes in the path for protocol extension methods

## MIR prerequisites

Before implementing the v0 mangler, the following MIR changes are needed:

1. **Store external labels in MIR function params** — currently only types are stored; the mangler needs labels to produce the `Z...E` signature
2. **Distinguish labeled vs unlabeled params** — MIR params need to record whether the external label is `_` (unlabeled) or a name
3. **Remove `$`-encoding from `name.rs`** — stop baking labels into qualified name segments; the mangler handles disambiguation via signatures
4. **Mangle statics properly** — replace `format!("{}", name)` in `context.rs:117` with a call to the mangler
5. **Store receiver convention in MIR** — methods need to record whether they take `Ref[Self]`, `RefMut[Self]`, or consuming `Self`, so the mangler can emit the `r`/`m`/`c` marker
