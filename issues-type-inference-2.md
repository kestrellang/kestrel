# Type Inference Issues — Round 2 (19 remaining)

Status: 19 errors at stdlib compilation (down from ~125 → 31 → 20 → 19).

Test command: `cargo test -p kestrel-compiler2 --release -- --nocapture compile_full_stdlib`

---

## Category A: Subscript `unchecked` label not resolving (5 errors)

All involve `array(unchecked: index)` returning `Array[T]` instead of `T`.

### A1. `Cursor.read` — expected UInt8 got Array[UInt8]
- **File:** `lang/std/io/read.ks:93`
- **Code:** `buf.pointer.offset(by: i).write(self.data(unchecked: self.pos + i));`
- `self.data` is `Array[UInt8]`, subscript should return `UInt8`

### A2. `Buffer.toString` — expected str got String
- **File:** `lang/std/io/write.ks:246`
- **Code:** `result.appendByte(self.data(unchecked: i));`
- `self.data` is `Array[UInt8]`, subscript should return `UInt8`
- Error message says `str got String` which may be a cascading mismatch

### A3. `DefaultStringInterpolation.build` — expected UInt8 got Array[UInt8]
- **File:** `lang/std/text/format.ks:274`
- **Code:** `totalBytes = totalBytes + self.parts(unchecked: j).byteCount;`
- `self.parts` is `Array[String]`, subscript should return `String`

### A4. `DefaultStringInterpolation.build` — expected String got Array[String]
- **File:** `lang/std/text/format.ks:280`
- **Code:** `result.append(self.parts(unchecked: i));`

### A5. `DefaultStringInterpolation.build` — NoMember 'byteCount'
- **File:** `lang/std/text/format.ks:274`
- **Cascading** from A3: receiver is `Array[String]` instead of `String`, so `byteCount` not found

**Root cause:** The subscript call `arr(unchecked: idx)` resolves the Array subscript with label `unchecked`, but the return type is computed as `Array[T]` (the whole array type) instead of `T` (the element). This suggests that in `solve_member`, the return type substitution for the subscript isn't applying struct type params correctly.

Specifically, the subscript's return type is `HirTy::Param(T_entity)` where `T` is Array's type parameter. The `subs` map in `solve_member` should map `T_entity → recv_type_arg[0]` (e.g., `UInt8`). If this mapping is missing or the wrong entity is used, the return type stays as `Named(Array_entity, [T_tv])`.

**Proposed fix:** Debug `solve_member` for subscript resolution on `Array[UInt8]` with label `unchecked`. Verify that `recv_type_args` extracts `[UInt8_tv]` and maps `T_entity → UInt8_tv` in `subs`. The subscript entity's parent is Array, so `TypeParams(Array)` should be used for the struct type param mapping.

**Files:** `lib2/kestrel-type-infer/src/solver.rs` (solve_member subs building)

---

## Category B: NoMember on chained subscript result (3 errors)

These are cascading from Category A — the subscript returns the wrong type, so subsequent member access fails.

### B1. `Grapheme.isAscii` — NoMember 'isAscii'
- **File:** `lang/std/text/char.ks:274`
- **Code:** `self._chars(unchecked: Int64(intLiteral: 0)).isAscii()`
- `self._chars` is `Array[Char]`, subscript should return `Char`, then `.isAscii()` resolves on `Char`

### B2. `Grapheme.utf8Length` — NoMember 'utf8Length'
- **File:** `lang/std/text/char.ks:284`
- **Code:** `self._chars(unchecked: i).utf8Length()`
- Same pattern as B1

### B3. `DefaultStringInterpolation.build` — NoMember 'byteCount'
- Same as A5, listed here for completeness

**Root cause:** Same as Category A. Once subscript returns the correct element type, these cascade errors resolve.

---

## Category C: Associated type not connecting through where clause (2 errors)

### C1. `sumValues` — expected V got V
- **File:** `lang/std/collections/dictionary.ks:1329`
- **Extension:** `extend Dictionary[K, V, H] where V: Addable, V.Output = V, V: Defaultable`
- **Code:** `result = result + value;`
- `result: V`, `value: V`, `+` resolves `Addable.add(other: Rhs) -> Output`
- Where clause says `V.Output = V`, so return type should be `V`

### C2. `sum` (Set) — expected T got T
- **File:** `lang/std/collections/set.ks:937`
- **Extension:** `extend Set[T, H] where T: Addable, T.Output = T, T: Defaultable`
- **Code:** `total = total.add(elem);`
- Same pattern as C1

**Root cause:** The `where_clause_assoc_subs` mapping (`Output_entity → V_tv`) is set up in `emit_extension_where_clauses`. When `solve_member` processes the `add` method's return type (which is `Output`), it calls `lower_hir_ty_sub` which checks `ctx.where_clause_assoc_subs`. However, the `Output` entity from the protocol method's return type may be a different entity than the one stored in `where_clause_assoc_subs`, OR the check is being bypassed because the entity has args or is processed through a different code path.

The "V got V" / "T got T" pattern means two different TyVars both represent the same type parameter but aren't unified. One comes from the extension's fresh type args, the other from the member resolution return type path.

**Proposed fix:** In `solve_member`, when building `subs` for protocol methods, include the extension's `where_clause_assoc_subs` entries so they get passed down to `lower_hir_ty_sub`. Alternatively, ensure the `Output` entity from the protocol definition matches the entity stored in `where_clause_assoc_subs`.

**Files:** `lib2/kestrel-type-infer/src/solver.rs` (solve_member, lower_hir_ty_sub), `lib2/kestrel-type-infer/src/lib.rs` (emit_extension_where_clauses)

---

## Category D: Numeric type mismatches (2 errors)

### D1. `TcpStream.detachFd` — expected Int32 got Int64
- **File:** `lang/std/net/socket.ks:109`
- **Code:** `self.fd = -1;`
- `self.fd: Int32`, but `-1` is an `Int64` literal
- No numeric literal coercion from Int64 to Int32

### D2. `Float32.toInt64` — expected Float32 got Float64
- **File:** `lang/std/num/float32.ks:987`
- **Code:** `if truncated < -9223372036854775808.0 {`
- `truncated: Float32` (from `self.trunc()` on Float32), but `9223372036854775808.0` is a Float64 literal
- No numeric literal coercion from Float64 to Float32

**Root cause:** The type inference system doesn't coerce numeric literals to narrower types. Integer literals default to Int64, float literals default to Float64. Assignment/comparison to Int32/Float32 fields doesn't trigger narrowing.

**Proposed fix:** Either:
1. Fix the stdlib code to use explicit typed values (e.g., `Int32(intLiteral: -1)`, typed float var)
2. Add numeric literal coercion in the solver (when a literal is assigned to a known narrower type)

Option 1 is simpler and avoids solver complexity. These are stdlib bugs, not compiler bugs.

**Files:** `lang/std/net/socket.ks`, `lang/std/num/float32.ks`

---

## Category E: Iterator adapter issues (4 errors)

### E1. `SkipWhileIterator.next` — DoesNotConform (Equal)
- **File:** `lang/std/iter/adapters.ks:153`
- **Code:** `if self.predicate(value) == false {`
- `predicate(value)` returns `Bool`, `== false` requires `Bool: Equatable`
- The solver can't find `Bool: Equatable` conformance in this context

**Root cause:** The `==` operator is resolved via protocol bounds. `Bool` does conform to `Equatable`, but the solver may not be finding this conformance when resolving `==` inside a generic iterator adapter. The conformance search may not be walking the concrete type's conformances when the receiver is the result of a closure call on a generic type.

### E2. `SkipWhileIterator.next` — NoMember 'equals'
- **File:** `lang/std/iter/adapters.ks:153`
- **Cascading** from E1: `==` resolves to `equals` method on `Equatable`, but since `DoesNotConform` fails first, the member lookup also fails.

### E3. `ScanIterator.next` — InfiniteType
- **File:** `lang/std/iter/adapters.ks:674`
- **Code:** `self.state = self.combine(self.state, item);`
- `combine: (Acc, I.Item) -> Acc`, `self.state: Acc`
- The solver detects infinite type, likely from `Acc` unifying with a function type or nested structure

**Root cause:** The `combine` field has type `(Acc, I.Item) -> Acc`. When the solver processes `self.combine(self.state, item)`, it may be unifying `combine`'s type with its return type, creating `Acc ~ (Acc, I.Item) -> Acc ~ ...`. This suggests the field access `self.combine` isn't being typed as a function, or the Call constraint is creating a recursive unification.

### E4/E5. `compactMap` — FromHir + TypeMismatch
- **File:** `lang/std/iter/iterator.ks:250`
- **Code:** `FilterMapIterator(inner: self, transform: { it })`
- **FromHir:** The closure `{ it }` uses implicit parameter `it` which may not be supported in Kestrel's HIR lowering
- **TypeMismatch:** `expected (Item) -> Optional[T] got () -> Error` — closure parsed as zero-param `() -> Error` instead of one-param closure

**Root cause:** `{ it }` is parsed as a block containing the expression `it` (a bare identifier). If `it` isn't a recognized implicit parameter or local variable, it becomes an unresolvable name → `Error` type. The closure has no declared parameters, so it's typed as `() -> Error`.

**Proposed fix:** Either:
1. Add implicit `it` parameter support for single-param closures (language feature)
2. Fix the stdlib to use explicit closure syntax: `{ (inner) in inner }`

Option 2 is simpler if implicit params aren't a planned feature.

**Files:** `lang/std/iter/iterator.ks` (stdlib fix), or parser/HIR lowering for implicit params

---

## Category F: Optional/Result issues (2 errors)

### F1. `Optional.expect` — expected Optional[?] got T
- **File:** `lang/std/result/optional.ks:148`
- **Code:**
  ```
  match self {
      .Some(value) => value,
      .None => lang.panic(message)
  }
  ```
- Return type is `T`. `.Some(value)` returns `T`, `.None => lang.panic(message)` returns `Never`.
- Error says "expected Optional[?] got T" — suggests the match is unifying with `self`'s type (`Optional[T]`) instead of the arm result type

**Root cause:** The match expression's type is being inferred from the scrutinee (`self: Optional[T]`) rather than from the arm results. Or `lang.panic` isn't returning `Never`, causing the match arms to not unify correctly.

### F2. `Optional.flatten` — TypeMismatch
- **File:** `lang/std/result/optional.ks:228`
- **Code:**
  ```
  public func flatten[U]() -> Optional[U] where T = Optional[U] {
      match self {
          .Some(inner) => inner,
          .None => Optional[U].None
      }
  }
  ```
- Where clause `T = Optional[U]` should make `inner` have type `Optional[U]`
- Error suggests `inner` has type `T` (not substituted via where clause)

**Root cause:** The where clause `T = Optional[U]` (type equality) may not be emitted as a constraint, or the match destructuring doesn't use the equality to determine `inner`'s type. When destructuring `.Some(inner)` from `Optional[T]`, `inner` gets type `T`. The where clause should equate `T` with `Optional[U]`, making `inner: Optional[U]`. If the equality constraint isn't emitted or solved before the match arm type check, `inner` stays as bare `T`.

**Proposed fix:** Ensure method-level where clauses with type equality (`where T = Optional[U]`) emit `equal(T_tv, Optional[U]_tv)` constraints at the start of method body inference. Check `create_param_types` and `instantiate_entity_inner` in generate.rs.

**Files:** `lib2/kestrel-type-infer/src/generate.rs` (where clause equality emission)

---

## Category G: FromHir / lowering issues (1 error)

### G1. `readlink` — FromHir
- **File:** `lang/std/os/fs.ks:313`
- **Code:** `let ch = UInt8(raw: lang.cast_i8_u8(byte));`
- The expression at the error offset involves `lang.cast_i8_u8(byte)` where `byte` comes from `lang.ptr_read(...)`

**Root cause:** The HIR lowering can't process the nested `lang.*` FFI calls. The `lang.ptr_read(lang.cast_ptr[lang.i8](lang.ptr_offset(buf, i.raw)))` chain may contain a path segment or type argument that the HIR lowerer doesn't handle. Alternatively, the `lang.cast_i8_u8` function may not be registered or resolvable.

**Proposed fix:** Check if `lang.cast_i8_u8` is defined in the lang module and if the name resolver finds it. The FromHir error means the expression couldn't be lowered at all, suggesting a resolution failure rather than a type issue.

**Files:** `lib2/kestrel-hir-lower/src/expr.rs`, `lang/std/os/fs.ks`

---

## Summary by priority

| Priority | Category | Errors | Impact |
|----------|----------|--------|--------|
| **High** | A+B: Subscript unchecked label | 7 | Single root cause, fixes 7 errors |
| **High** | C: Associated type where clause | 2 | Continuation of prior where clause work |
| **Medium** | E: Iterator adapters | 4 | Mixed causes, some may be stdlib fixes |
| **Medium** | F: Optional/Result | 2 | Where clause equality + match typing |
| **Low** | D: Numeric coercion | 2 | Stdlib fixes (not compiler bugs) |
| **Low** | G: FromHir readlink | 1 | Likely name resolution issue |
