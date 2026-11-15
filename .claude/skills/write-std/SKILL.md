---
name: write-std
description: 'How to add to or change the Kestrel standard library in lang/std: layout, visibility, lang items (@builtin), intrinsics, FFI and C shims, platform code, copy-on-write storage, docs and tests. Use for any edit under lang/std/.'
---

# Writing the std

The standard library is ordinary Kestrel in `lang/std/`. For the language
itself use the `write-kestrel` skill; this one covers what is different about
the std.

## Layout

- **One module per directory.** `lang/std/collections/` is `std.collections`,
  and every file in it starts with `module std.collections`. A new directory is
  a new module; nothing needs registering.
- **File names are lowercase `snake_case.ks`**, after the main type or topic
  (`deque.ks`, `slice_methods.ks`).
- **Each file lists its imports explicitly**, by convention:
  `import std.numeric.(Int64)`, `import std.memory.(Pointer, CowBox)`.
- Two levels of module is normal, three is the limit (`std.text.unicode`).
- Names follow `docs/NAMING_CONVENTIONS.md`: no abbreviations, mutating verbs
  vs past-participle copies (`sort` / `sorted`), `to*` converts and `as*` views.

## The edit loop

The compiler reads the std from `lang/std` when it runs, so a `.ks` change
needs no `cargo build`. Build the compiler only when you change Rust.

1. Write a small probe program that uses the new API and build it with
   `target/release/kestrel build probe.ks -o probe`.
2. **Every test compiles the whole std**, so one std error fails the entire
   suite. Get the probe building before you run any tests.
3. `KESTREL_STD=<dir>` points the compiler at another std copy, which is
   handy for trying an experimental change without touching the tree.

## Visibility

| Keyword | Visible to |
|---|---|
| `public` | Every module. **Every public std item is auto-imported into user code**, so a new public name can collide with a user's own; pick it with care |
| `internal` (the default) | Everything under the same top-level module, which for the std is all of `std`, not just its own module |
| `fileprivate` | Members: the same file |
| `private` | Members: the declaring type |

Top-level `private` and `fileprivate` functions and types are not
file-scoped yet; another file of the same module can still use them.
`@doc(.Hidden)` keeps a public item out of the docs and completion when it has
to be public only for the compiler or another std module.

## Lang items: `@builtin`

When the compiler needs a std declaration (a protocol it desugars to, a type
literals default to, a method it calls), tag it with `@builtin(.Name)`:

```kestrel
@builtin(.EquatableProtocol)
public protocol Equatable { … }
```

- The compiler finds it through `ResolveBuiltin`, never by its name. Renaming
  or moving a tagged item is safe; removing the tag breaks the compiler.
- `@builtin` is only allowed in the std (E401), and each tag may appear once
  (E400).
- A new lang item is a compiler change too: a variant in
  `lib/kestrel-ast/src/builtin.rs` (the enum, `name()`, `from_attribute_name()`
  and `kind()`), then a `ResolveBuiltin` call where the compiler uses it.

## Intrinsics: `lang.*`

The `lang` module holds the primitive types (`lang.i64`, `lang.f64`,
`lang.ptr[T]`, `lang.str`) and intrinsic functions (`lang.i64_add`,
`lang.cast_i64_i32`, `lang.is_bitwise[T]()`). Only the std's lowest layers use
them, wrapped in proper types:

```kestrel
if Bool(boolLiteral: lang.is_bitwise[T]()) { … }   // lang.i1 → Bool
```

Prefer writing the operation in Kestrel. A new intrinsic is a compiler change:
seed it in `lib/kestrel-ast-builder/src/lang_module.rs` and lower it in
`lib/kestrel-mir-lower/src/body/call/intrinsic.rs`.

## Unsafe code and C

- Raw memory goes through `std.memory` (`Pointer[T]`, `Layout`, `Allocator`),
  and every unsafe call sits inside `unsafe` (`docs/language/unsafe.md`).
  Mark a function `unsafe` when its caller must uphold a contract, and say what
  the contract is in its docs.
- C functions are declared with `@extern(.C, mangleName: "free")`. Calling one
  always needs `unsafe`, and its parameter and return types must be `FFISafe`.
- C a module needs beyond libc goes in a shim file such as
  `files/files_shims.c`, listed in `STDLIB_C_SHIMS` in
  `lib/kestrel-compiler/src/stdlib_path.rs` so every build links it.

## Platform code and data

- `@platform(.Linux)` and `@platform(.Darwin)` give one declaration per OS;
  the other is dropped before name resolution:

  ```kestrel
  @platform(.Darwin)
  internal func currentOperatingSystem() -> OperatingSystem { .Darwin }

  @platform(.Linux)
  internal func currentOperatingSystem() -> OperatingSystem { .Linux }
  ```

- `@fileconstant("data/table.bin")` embeds a file, relative to the source file,
  as a module-level `let` (the Unicode tables use it).

## Other attributes the std uses

| Attribute | Use |
|---|---|
| `@inline(.Always)` / `@inline(.Never)` | Keep a fast path inline and push the slow path (growth, a fork) out of line. LLVM only |
| `@observabledrop` | A type whose drop has an effect (closing a socket), so its drops stay at scope end. Missing it on such a type gives warning E508 |
| `@sugarOnly` | A protocol requirement reached only through syntax (`+=`, `try`, literals). Code outside its module can't call it by name |
| `@existentialOnly` | A protocol-extension default only `any P` uses |
| `@nonExhaustive` | A public enum that may gain cases; outside matches need a catch-all |
| `@deprecated(…)` / `@unavailable(…)` | Retire an API with a warning (E908) or an error (E913) |
| `@derive(…)`, `@layout(.C)`, `@align(N)` | As in user code |

## Copy-on-write types

A collection holds its buffer in a storage struct behind `CowBox`, so copying
the collection bumps a refcount and the first write to a shared copy forks it.
`lang/std/collections/deque.ks` is the model to copy:

- The storage struct is `Cloneable`. Its `clone()` makes a deep copy, and its
  `deinit` drops every live element (`dropInPlace`) and then frees the buffer.
- The public type stores `private var storage: CowBox[Storage]`.
- Reads go through `self.storage.value`, which never forks. Writes go through
  `value`'s `mutating ref` (`self.storage.value.len += 1`, or
  `let s = &mutating self.storage.value`), which forks shared storage first.
- To keep the hot path small, check `self.storage.isUnique` inline and call an
  out-of-line slow path for growth or the fork (see `Array.append`).
- Collections take an allocator parameter, `A = GlobalAllocator`.

Box types that may hold an existential have extra rules, listed in
`lang/std/memory/AGENTS.md`.

## API shape

- Take `&Slice[T]` and `&StringSlice` rather than `Array` and `String` for
  read-only sequence parameters, so callers can pass any view
  (`docs/language/references.md`).
- A wrapper that should act like its contents conforms to `Indirection`
  (`docs/language/indirection.md`); don't forward members one by one.
- Return a view (`&StringSlice`) when the result borrows from `self`, and let
  `DecaysFrom` turn it into an owned value where the caller asks for one.

## Docs and tests

1. **Document every public item** with `///`: a summary, details, Safety and
   Errors sections where they apply, and an Example. The `expand-docs` skill
   has the house style, including `@name` tags for inits and subscripts.
2. **Regenerate the reference** with the `stdlib-docs` skill. `docs/stdlib/`
   is generated; don't edit it by hand.
3. **Add an execution test** under
   `lib/kestrel-test-suite/testdata/stdlib/<type>/`. Check output with
   `// expect-stdout:` or `// stdout-contains:`; the harness doesn't read a
   `// CHECK:` line. The `write-tests` skill has the format.
4. Run the tests through the `triage` skill.

## Checklist

- [ ] The probe builds and runs with a fresh `kestrel`.
- [ ] New public names fit the naming rules and don't collide with common user names.
- [ ] Every `unsafe` use is in an `unsafe` expression with its contract documented.
- [ ] A COW write goes through `mutating ref`; a read doesn't fork.
- [ ] Doc comments are written and the reference is regenerated.
- [ ] An execution test covers the new behavior.
