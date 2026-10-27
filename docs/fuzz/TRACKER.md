# Fix tracker

Status: **done** / todo / arch (architectural, later) / not-a-bug / decision (needs your call)

| ID | Status | Bug | Commit |
|---|---|---|---|
| C1 | **done** | Integer literal and range patterns truncated to 32 bits (`5000000000` matches `705032704`, `0..=i64max` misses | |
| C2 | **done** | Assignment through `arr(i).field.x` or `arr(i).0` silently dropped | |
| C3 | **done** | Tuple rest pattern `(a, .., b)` binds the wrong element | |
| C4 | **done** | Labels in enum payload patterns are ignored; fields bind by position | |
| C5 | **done** | Two conformances to one generic protocol (`Conv[Int64]`, `Conv[String]`): generic dispatch segfaults | |
| C6 | **done** | Same setup, direct call overloaded only by return type: segfault | |
| C7 | **done** | Closure parameter returning a String concat corrupts the heap (`malloc(): unaligned tcache chunk`) | |
| C8 | **done** | `consuming` closure loses writes to a captured var's fields; an Array append hits the outer buffer | |
| C9 | not-a-bug | `escaping` closure that captures a not-Copyable local reads it after it is dropped | |
| C10 | **done** | Assigning a field inside `deinit` drops the old value twice and leaks the new one | |
| C11 | **done** | Conformance accepted with the wrong parameter type; the call passes Int64 as String and segfaults | |
| C12 | **done** | `mutating` impl satisfies a non-mutating requirement, so a `let` binding gets mutated | |
| C13 | **done** | Moving out of a `mutating` parameter accepted, so a double drop | |
| C14 | **done** | `deinit r` on a borrowed parameter accepted, so a double drop | |
| C15 | **done** | `consuming` closure built in a loop moves its capture on every iteration, so a triple drop | |
| C16 | **done** (array literals must be Copyable, like `Array[T]`) | Array literal of not-Copyable elements accepted; `for in` traps with SIGILL | |
| C17 | **done** | A ref-carrying struct stored into an outer `var` escapes its scope (dangling) | |
| C18 | **done** | View closure escapes inside an `Optional`, so a segfault | |
| C19 | **done** | View closure escapes through an `if` expression join, so a segfault | |
| C20 | **done** | View closure escapes through reassignment inside an `if` or loop, so a segfault | |
| C21 | **done** (literal defaults get annotation well-formedness) | View closure escapes through an un-annotated array literal, so a segfault | |
| C22 | **done** | View closure escapes through a call result, so a segfault | |
| C23 | **done** (E511, option 1) | Moving a field out of a type with `deinit` silently skips the owner's deinit | |
| C24 | **done** | Discarded temporary dropped at scope end instead of statement end (violates drop-semantics.md rule 6) | |
| C25 | **done** | Cranelift picks the wrong `conv` overload, giving a verifier error (LLVM builds it; probably the same root cau | |
| H1 | **done** | Bootstrap: `Iterator.iter()` instantiated at `EnumerateIterator<non-Copyable>` fails post-mono verify (**block | cd92125f |
| H2 | **done** | Nested struct or enum inside a struct (generic or not): `has no MonoStruct/MonoEnum`; nested enum case panics | f40691bb (E634 for capturing nested generics) |
| H3 | **done** | Generic struct with an `escaping` closure field over an associated type: "Copyable contains non-Copyable field | |
| H4 | **done** (E498 for named refs, new E510 for `f(x, x)`) | Moving a value while a borrow is live reaches the OSSA verifier instead of E498 (also `f(x, x)` with mutating  | |
| H5 | **done** | `while let` over a not-Copyable var reassigned or broken out of in the body: "address already uninit" | |
| H6 | **done** | String literal patterns on an owned scrutinee (match, tuple, Optional, array): "consumed more than once" or "n | |
| H7 | **done** | Matching a not-Copyable value twice gives an OSSA ICE instead of E500 | |
| H8 | **done** | `.Some(var r)` binding of a not-Copyable payload | |
| H9 | **done** | Not-Copyable match where every arm returns, inside `for`: Cranelift jump argument mismatch / LLVM verify failu | |
| H10 | **done** | Same loop shape with the match under `if`: "@owned never consumed" | |
| H11 | **done** | Struct patterns in match arms: "TupleExtract on non-tuple" | |
| H12 | **done** (closure args snapshot; E635 narrowed) | `two(x, { x })` with a mutating first argument: "active mut borrow" OSSA ICE | 3196cff8, 8c5d3191 |
| H13 | **done** | `gen[T: Cloneable]` instantiated at an escaping closure: post-mono "does not implement clone" | |
| H14 | **done** | Stored `var` in an `extend` block accepted, then Cranelift panics with index out of bounds | |
| H15 | **done** | `struct S { var x: S? }` fails post-mono verify (should be E-diagnostic or work) | |
| H16 | **done** | `deinit r;` as the only statement panics in move_tracking.rs:2278 | |
| H17 | **done** | Implicit member call to an instance method `g(.round())`: OSSA argument-count ICE | |
| H18 | **done** | `@main` with a parameter: OSSA argument-count ICE instead of a diagnostic | |
| H19 | **done** | `S.v` (instance field through the type name) reaches codegen: "global not found in statics" | |
| H20 | **done** (Conforms carries protocol args) | `Int8(from: Int8)` passes type-check and then fails post-mono verify | 2218629e |
| H21 | **done** | `return return` gives OSSA "operand never defined" | |
| H22 | **done** | Polymorphic recursion `recur([x], n-1)` hangs the compiler | |
| H23 | **done** | Type alias cycle (`type A = B; type B = A`, `type A = S[A]`) overflows the compiler's stack; `type A = Array[A | |
| H24 | **done** | Default argument calling its own function overflows the compiler's stack | |
| H25 | **done** | Bool-tuple match compile time doubles per element (15 elements = 128s) | |
| H26 | **done** | `Float64.round()` rounds ties to even; the docs say away from zero | |
| M1 | **done** | Exhaustive Bool and enum tuple matches reported as missing `_` | |
| M2 | **done** | Full unsigned ranges (`0..=255` on UInt8) not exhaustive | |
| M3 | **done** | Empty match on an empty enum: E304 "inhabited" | |
| M4 | **done** | Struct pattern inside an enum payload `.Some(P { x, y })` is a parse error | |
| M5 | **done** | `deinit x;` only parses at the top level of a function body | |
| M6 | **done** (partial moves) | Field move from an owned local (`let x = p.a`) and tuple `p.0` from a consuming param rejected as "out of a bo | 420b1fa8, b6a1717c, 923ad3c9 |
| M7 | **done** | `match r` on a `&E` binding rejected even though the arms only read | |
| M8 | **done** (field-granular freezes) | E507 freeze is not field-granular (consuming `p.b` is blocked by a capture of `p.a.v`) | b6a1717c, 529e7262 |
| M9 | **done** | E507 still active after an immediately-invoked closure | |
| M10 | **done** | Sub-protocol extension can't see parent protocol methods | cd92125f |
| M11 | **done** | Protocol extension can't pass `self` to a `where T: P` function | cd92125f |
| M12 | **done** | `Box.make(3)` on an unspecialized generic leaks the rigid `T` | |
| M13 | **done** (rejected with E634) | Nested generic loses the outer type parameter | 81fe0af5 |
| M14 | **done** | `T.A = U.A` same-type constraint doesn't carry conformance | |
| M15 | **done** | Generic tuple alias `type Pair[T] = (T, T)` not expanded (documented in write-kestrel) | |
| M16 | **done** | `extend [T]: P` sugar fails where `extend Array[T]` works | |
| M17 | not-a-bug | `"\(x[0])"` subscript in interpolation is a parse error | |
| M18 | **done** | `t.1.0` nested tuple index lexed as a float | |
| M19 | **done** | Payload pattern with a wrong or duplicate label accepted (`.A(z: n)` for `A(x:)`) | |
| M20 | **done** | `let g: Int64 = g` and mutually recursive globals accepted | |
| M21 | **done** | `extend Int64: NopeProto {}` (undeclared protocol) accepted | |
| M22 | **done** (E629; `import A.B.T` imports a member) | `import Nope.Thing` accepted | |
| M23 | **done** | `E.a.b` (case access on a case) type-checks as `()` | |
| M24 | **done** | `struct S: S {}` accepted | |
| M25 | **partial**: two quadratic spots fixed (10,000 `let`s: 99s → 47s); `owned_params` still does linear scans per rewrite site and needs restructuring | 10,000 `let`s take 99s to compile (quadratic) | |
| M26 | **done** | `Float64.abs(-0.0)` returns `-0` | |
| M27 | **done** | Unterminated block comment: no error, only E618 pointed at a stdlib file | |
| L1 | **done** | Compiler stack overflow on 3,000-deep blocks, if-chains, closures or whiles (move_tracking) | |
| L2 | **done** | Compiler stack overflow on 3,000-deep binops, calls, method chains or matches (MIR lowering) | |
| L3 | **done** | Compiler stack overflow on a deeply nested tuple type | |
| L4 | **done** | `Array[...]` nested 3,000 deep runs out of memory | |
| L5 | **done** | Inference depth limit around 512 gives a bogus E109 | |
| L6 | **not a bug** (duplicate labels at different positions are allowed) | Duplicate external labels `f(a x:, a y:)` accepted | |
| L7 | **done** | Unknown attribute `@nope` silently accepted | |
| L8 | **done** | Format specifiers ignored for Bool width and float hex | |
| L9 | **done** | Float `:.N` precision switches between decimals and significant digits | |
| L10 | **done** | `Int64.minValue.gcd(6)` returns -2 | |
| L11 | **done** | Backtick identifier error leaks the internal name `$let_tmp` | |
| L12 | **done** | `(return 1) + 2` gives "no member 'add' on Never" | |
| L13 | **done** | UTF-8 BOM at file start rejected | |
| L14 | **done** | Negative literal patterns `-1 =>` unsupported | |
| A1 | **won't fix** (by design: plan D5 rejects this; see escaping_param_truncated_to_normal_cannot_be_returned.ks) | Returning an `escaping` closure parameter is rejected: `func adapt(f: escaping (Int64)->Int64) -> (Int64)->Int | |
| A2 | **done** | A generic call to a protocol-extension method that is not a requirement dispatches to the concrete type's same | |
| A3 | **done** | E494 on an `if`/`else` returning refs to two locals prints one error and then "OSSA verification failed with 2 | 67e6e96c |
| N1 | **done** | E503 'move out of a borrow' (e.g. returning `self` from a borrowing method) has no source location | |
| N2 | **done** | Borrowing witness for a consuming requirement leaked (or crashed) the owned argument; exposed by consuming `iter()` | cd92125f |
| N3 | **done** (concrete conformers; blanket protocol conformances still use clause entailment) | A witness with a stricter `where` than its requirement is accepted; the bound violation becomes a runtime trap (mono #127 backstop) instead of a compile error | |
| P1 | decision | `protocol_ext_witness_gated_by_{copyable,cloneable}_bound.ks` copy abstract `Self`; now correctly rejected. Update with `where Self: Copyable`? | |
| N4 | **done** | A local `escaping` closure returned as a normal `(Int64) -> Int64` is accepted and dangles (`func f() -> (Int64) -> Int64 { let k = 3; let g: escaping (Int64) -> Int64 = { (x) in x + k }; g }` returns a wrong value). Existed before these fixes. | |
