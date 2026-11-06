# Frontend bugs: root-cause analysis (no fixes)

Base: `/home/user/kestrel`, commit `3fea2041`, prebuilt `./target/release/kestrel` (not rebuilt).
Every repro under `temp/fuzz/frontend/min/` was re-run to get the symptoms below. Backtraces come from gdb on the release binary.

---

## Group A: nested nominal types are never lowered to MIR (H2, all parts)
Repros: `nested_struct_basic`, `nested_same_name`, `nested_enum_case`, `generics_protocols/min/nested_struct_in_generic_ice`.

- **Root cause:** the MIR item walk recurses only into `Module` children. A `struct`/`enum` declared inside another type never gets `lower_struct`/`lower_enum`. So there is no `MirStruct`/`MirEnum` (post-mono verify reports "has no MonoStruct/MonoEnum"), no variant table (the `nested_enum_case` panic "variant 'a' not found"), and no lowering of its methods.
- **Location:** `lib/kestrel-mir-lower/src/items/mod.rs:50-63` (`lower_types`), which has no `Struct|Enum|Extension => recurse` arm. `lower_member_functions` at `items/mod.rs:94-122` has the same gap: it never descends into a nested type's members. The enum panic fires at `lib/kestrel-mir-lower/src/body/expr.rs:975-981` (`lower_expr_inner`, `Some(NodeKind::EnumCase)`).
- **Why it's missed:** both walkers assume types exist only at module scope.
- **Generic part:** the mangled name shows `Outer.Inner[Int64]` (the nested type takes on the outer type's arguments), but the type is `Named(Inner, [])`. Once lowering recurses, check how nested types in generic parents are parameterized.
- **Confidence:** high.
- **Fix shape:** recurse into `Struct`/`Enum` children in `lower_types` and into nested types in `lower_member_functions`.

## H14: stored `var` in `extend` is accepted, then Cranelift panics
- **Root cause:** no analyzer rejects a stored (non-computed, non-static) field in an extension. The struct layout comes only from the struct body, so `s.x` lowers to a `StructExtract` index into an empty struct and indexes out of bounds in `struct_field_type` (`lib/kestrel-codegen-cranelift/src/inst.rs:2156`).
- **Location:** the check is missing from `lib/kestrel-analyze/src/decl/extension_validation.rs`, which only checks the target (E452) and the type-arg count (E453). A computed `var x: Int64 { 3 }` in the extension works.
- **Confidence:** high.
- **Fix shape:** add an E45x "extensions cannot contain stored properties" check for `Field`s in an `Extension` that are neither `Callable`/computed nor `Static`.

## H15: `struct S { var x: S? }` gives a post-mono verify ICE
- **Root cause:** `Optional` is a plain non-`indirect` enum (`lang/std/result/optional.ks:57`) stored inline, so `S` contains itself by value. The struct-cycle analyzer treats every non-struct `Named` type, and `Optional`/`Array`/etc. in general, as heap indirection and stops there.
- **Location:** `lib/kestrel-analyze/src/compilation/struct_cycles.rs:200-227` (`check_type`): only `NodeKind::Struct` is followed, generic arguments are never walked, and the `_ => Ok(())` arm says "Optional … indirection". `decl/recursive_enum.rs` (header comment "Stops at … Optional") has the same false assumption.
- **Confidence:** high.
- **Fix shape:** follow generic arguments of non-`indirect` value types (non-indirect enums such as Optional/Result, and generic structs) in the cycle walk. Stop only at genuinely boxed types (Array, Pointer, indirect enums, functions).

## H17: `g(.round())` (implicit member naming an instance method) gives an OSSA arg-count ICE
- **Root cause:** implicit-member resolution tries *instance* members first and accepts an instance method with no receiver. It binds `Float64.round` as if it were static/case-like, so MIR emits a 0-arg call to a function that takes `self`.
- **Location:** `lib/kestrel-type-infer/src/solver/member.rs:1219-1236` (`solve_implicit`): `resolve_member` succeeds with no staticness filter. The only post-check is the enum-case label check at 1240.
- **Confidence:** high.
- **Fix shape:** in `solve_implicit`, accept only static members, enum cases or initializers. Otherwise report "implicit member must be static" (as `ImplicitMemberNotFound`/`MemberIsStatic`).

## H19: `S.v` (instance field through the type name) reaches codegen ("global not found in statics")
- **Root cause:** name resolution returns `Def(field)` for `[S, v]`. HIR lowering rejects only *instance methods* named through a type. A non-static field passes through as `HirExpr::Def(field)`, inference gives it the field's type, and MIR treats a `Def` of a `Field` as a static global.
- **Location:**
  - `lib/kestrel-hir-lower/src/expr.rs:487-495` (path lowering; the guard uses `is_instance_method`, defined at `expr.rs:1105-1114`, which matches `NodeKind::Function` only).
  - `lib/kestrel-name-res/src/resolve_value.rs:395` (`walk_path_from` returns `Def(current)` with no staticness check).
- **Confidence:** high.
- **Fix shape:** extend the guard to non-`Static` `Field`s (an "instance member on type" diagnostic), sharing `emit_instance_method_on_type`.

## H21: `return return` / `return return 1` gives OSSA "operand never defined"
- **Root cause:** `HirExpr::Return` lowers its value (the inner `return`), which terminates the block. It then calls `emit_ret` on the already-terminated block. `set_terminator` overwrites the first `Return` with a value defined only after the terminator, and the inner value is left unconsumed.
- **Location:** `lib/kestrel-mir-lower/src/body/expr.rs:550-583` (`lower_expr_inner`, `HirExpr::Return` arm). There is no `self.is_terminated()` check after lowering `value`. Other sites already use that check, e.g. `body/control.rs:257`.
- **Confidence:** high.
- **Fix shape:** after lowering the value, if `is_terminated()`, skip the drops and the `emit_ret` and return an unreachable/unit value.

## H23: type-alias cycles overflow the stack; `type A = Array[A]` is silently accepted
- **Root cause (overflow):** HIR type lowering eagerly expands "trivial" aliases by recursing into their annotation with no visited set. `A=B; B=A` and `A = S[A]` (through the type-argument lowering) recurse forever before the cycle analyzer (E447) can report. gdb shows 39+ nested `kestrel_hir_lower::ty::lower_ast_type` frames.
- **Location:** `lib/kestrel-hir-lower/src/ty.rs:290-295` (`build_hir_ty_for_entity` → `lower_ast_type(…, &ann.0)` for `is_trivial_alias`), entered from `ty.rs:30` (`lower_ast_type`).
- **Root cause (silent accept):** `lib/kestrel-analyze/src/compilation/type_alias_cycles.rs:138-179` (`check_type`) never walks a `Named` type's generic arguments, and its `_ => Ok(())` treats Array/Optional as "nominal indirection". Aliases are transparent, though, so `A = Array[A]` is an infinite type. `alias_generic_cycle` passes only because `A` is never used.
- **Confidence:** high.
- **Fix shape:**
  - Thread an in-progress alias set through `lower_ast_type`'s alias expansion: on re-entry, emit E447 and return `HirTy::Error`.
  - Walk type arguments in the E447 analyzer.

## H24: a default argument that calls its own function overflows the stack
- **Root cause:** default arguments are inlined at each call site by re-lowering the default's HIR body. When the default expression calls `f()` with no argument, `lower_default_arg_inline` recurses into itself without end. The code comment claims nested default lowering "is not possible".
- **Location:** `lib/kestrel-mir-lower/src/body/call/args.rs:249` (`lower_default_arg_inline`, comment at ~284), reached from `args.rs:155/213` (`lower_call_args_bound`). gdb shows the `lower_expr → lower_call_args_bound → lower_default_arg_inline` cycle.
- **Confidence:** high.
- **Fix shape:** keep an "inlining defaults" stack of `default_entity`s and diagnose re-entry, or reject in an analyzer any default expression that transitively uses its own parameter's default.

## Group B: compiler recursion with no stack guard (L1, L2, L3)
Only the parser uses `stacker::maybe_grow` (`lib/kestrel-parser/src/parser.rs:138`). The later recursive passes have no guard. Frames sampled with gdb:

- **L1** (`block_deep`, `deep_if_chain`, `deep_closure_nest`, `deep_while_nest`, `unary_deep`): `kestrel_analyze::body::move_tracking::analyze_expr` ↔ `analyze_block`. Location: `lib/kestrel-analyze/src/body/move_tracking.rs:430` (`analyze_expr`).
- **L2** (`binop_deep`, `deep_call_nest`, `deep_method_chain`, `deep_match_nest`): `OssaBodyCtx::lower_expr` → `lower_expr_inner` → `prepare_call_arg_for_expr` / `lower_call_args_bound` / `emit_decision_tree_threaded` → `emit_success_leaf`. Location: `lib/kestrel-mir-lower/src/body/expr.rs:32` (`lower_expr`).
- **L3** (`deep_tuple_nest`): `kestrel_mir_lower::ty::lower_resolved_ty_preserving`, which recurses on the tuple type. Location: `lib/kestrel-mir-lower/src/ty.rs:209`.
- **Confidence:** high.
- **Fix shape:** wrap these recursion points in `stacker::maybe_grow` (as `parser.rs:138` does), or add a nesting-depth limit with a diagnostic.

## L4: `Array[...]` nested 3,000 deep runs out of memory
- **Root cause:** symbol size grows quadratically. Each nesting level is its own monomorphized instantiation, and each instantiation's mangled name spells out the whole nested type with no back-references. Total symbol bytes are therefore O(depth²) per method.
- **Measurements:**
  - depth 100: an 8.5 MB binary with 9.2 KB symbols.
  - depth 300: 61 MB.
  - depth 800: 3 GB RSS, sampled inside `object::write::StringTable::write` from `CodegenCtx::finish`.
- **Location:** `lib/kestrel-mir/src/mono/mangle.rs:121` (`mangle_type`, no substitution or back-reference compression). The string table is written at `lib/kestrel-codegen-cranelift` `CodegenCtx::finish`.
- **Confidence:** medium.
- **Fix shape:** add Itanium-style substitutions to the mangler (or hash long symbols), and/or a type-nesting depth limit.

## L5: E109 "expected _ got Int64" at about 512 unary minuses
- **Root cause:** the fixpoint solver caps at 256 rounds, and a chain of deferred `neg` member constraints settles only about one or two links per round. At around 512 nested operators the cap is hit with vars still unresolved, and the leftovers are reported as a bogus mismatch. Depths of 100–511 compile.
- **Location:** `lib/kestrel-type-infer/src/solver/mod.rs:435-444` (`fixpoint`, `MAX_ROUNDS = 256`).
- **Confidence:** medium-high.
- **Fix shape:** drive solving by a worklist that wakes constraints on var resolution, or scale the cap with the number of constraints, instead of a fixed round count.

## L6: duplicate external labels `f(a x:, a y:)` are accepted
- **Root cause:** the duplicate-label analyzer checks only enum-case payloads.
- **Location:** `lib/kestrel-analyze/src/decl/duplicate_label.rs:50-88` (`check` iterates `NodeKind::EnumCase` children only).
- **Note:** Swift allows duplicate argument labels, so this is a language decision more than a bug.
- **Confidence:** high.
- **Fix shape:** if this should be rejected, extend the check to the `Callable` of functions, inits and subscripts.

## L7: unknown attribute `@nope` is "silently" accepted
- **Root cause:** the diagnostic exists (E461, a *Warning*; `lib/kestrel-analyze/src/compilation/unknown_attribute.rs:42-88`) and `kestrel dump diagnostics` shows it. `kestrel build` calls `driver.emit_diagnostics()`, which drops all analyzer warnings.
- **Location:** `src/main.rs:276` → `lib/kestrel-compiler-driver/src/lib.rs:228` (`emit(false)`).
- **Confidence:** high.
- **Fix shape:** have `build` show analyzer warnings (or a set of always-on warnings), or make E461 an error.

## L8: format specifiers ignored for Bool width and float hex
- **Root cause:** stdlib `format` implementations ignore the options.
  - `Bool.format` looks only at `options.debug`, so width/fill/align are dropped (`lang/std/core/bool.ks:136-145`).
  - `Float64.format` (and Float32) look at precision/floatStyle/sign/width but never radix, so `:x` on a float prints decimal (`lang/std/numeric/float64.ks:1110`, `float32.ks:1110`).
- **Confidence:** high.
- **Fix shape:** route Bool through the shared padding helper the integers use. For floats, either implement radix formatting or reject an integer-only spec on non-integer operands.

## L11: a backtick identifier error leaks `$let_tmp`
- **Root cause:**
  - The lexer has no backtick raw-identifier token, so `` ` `` becomes "unexpected character" and `let` is left with no pattern.
  - `lower_let_stmt` sends a missing pattern down the destructuring path (`binding == None`). That creates the synthetic `$let_tmp` local with no value.
  - Definite-assignment then reports E004 "access to uninitialized variable '$let_tmp'" using the synthetic name. A spurious E105 comes from the same recovery.
- **Location:**
  - `lib/kestrel-hir-lower/src/stmt.rs:107-120` (binding detection) and `stmt.rs:190` (`define_local("$let_tmp", …)`).
  - `lib/kestrel-analyze/src/body/definite_assignment.rs:~180` (uses `locals[id].name` with no `$`-synthetic filter).
  - Lexer: `lib/kestrel-lexer/src/lib.rs:174` (Identifier regex only).
- **Confidence:** high.
- **Fix shape:**
  - When the `let` pattern is missing or an error, lower it to an error stmt instead of `$let_tmp`.
  - Have E004 and other analyzers skip `$`-prefixed synthetic locals.
  - Optionally support `` `ident` `` in the lexer.

## L12: `(return 1) + 2` gives "no member 'add' on Never"
- **Root cause:** the operator desugars to `ProtocolCall` → `Conforms(recv, Addable)` + `Member(recv, "add")`. `solve_member` absorbs only `Error` receivers. A `Never` (diverging) receiver goes to `resolve_member`, which looks up the `lang "!"` entity, finds no `add`, and reports E102/E107.
- **Location:**
  - `lib/kestrel-type-infer/src/solver/member.rs:335-345` (`solve_member`, which only has the `is_error` short-circuit).
  - The conformance check for the same receiver is emitted at `lib/kestrel-type-infer/src/generate.rs:533-537`.
- **Confidence:** medium-high.
- **Fix shape:** treat a resolved `TyKind::Never` receiver like `Error` in `solve_member` and in conforms solving: the result is Never/poisoned, with no diagnostic. A "will never be executed" warning is optional.

## L13: a UTF-8 BOM at file start is rejected
- **Root cause:** nothing strips U+FEFF. The source is read verbatim, and no lexer token matches the BOM: `Whitespace` is `[ \t\f]+`.
- **Location:**
  - `lib/kestrel-lexer/src/lib.rs:155` (Whitespace regex) and `lib/kestrel-lexer/src/modal.rs:53` (`lex_modal`).
  - Source reads at `lib/kestrel-compiler/src/lib.rs:561` and `src/main.rs:492/558`.
- **Confidence:** high.
- **Fix shape:** at the start of `lex_modal`, emit a leading `\u{FEFF}` as a trivia (Whitespace) token so spans stay correct.

## M17: `"\(x[0])"` subscript in interpolation is a parse error
- **Not a bug:** this is invalid Kestrel. Subscripts are written `x(0)`, and brackets are for type arguments only (`.claude/skills/write-kestrel/SKILL.md:564`).
- **Evidence:** `let y = x[0]` outside a string fails the same way (E801), and `"\(x(0))"` compiles and prints `1`.
- **Fix shape:** at most, a better diagnostic ("subscripts use `x(i)`").

## M20: `let g: Int64 = g` and mutually recursive globals are accepted
- **Root cause:** global initializers become per-static init thunks, all run eagerly from `__kestrel_init_statics` in `module.statics` order. Nothing builds a dependency graph between static initializers, so a self-reference or a cycle reads zero-initialized storage (`g` prints `0`).
- **Location:**
  - `lib/kestrel-mir-lower/src/items/static_lower.rs:48-75` (`synthesize_static_inits`, no ordering or cycle check).
  - No analyzer exists under `lib/kestrel-analyze/src/compilation/` for static-init cycles.
- **Confidence:** medium-high.
- **Fix shape:**
  - Add a compilation analyzer that builds the static→static reference graph (through called functions, if wanted) and reports cycles.
  - Order init thunks topologically.

## Group C: conformance-list entries are never validated (M21, M24)
Repros: `extend Int64: NopeProto {}` (M21), `struct S: S {}` (M24). I also confirmed `struct Q: NopeProto {}` is silently accepted, and so is `struct R: Q {}` where `Q` is a struct.

- **Root cause:** conformance resolution silently `filter_map`s away entries that don't resolve and keeps only results whose `NodeKind` is `Protocol`. No analyzer reports an unresolved or non-protocol conformance entry. Generic bounds do have one: E436 at `lib/kestrel-analyze/src/decl/generics.rs:~660-680`.
- **Location:** `lib/kestrel-name-res/src/conformances.rs:306-326` (`declared_conformance_protocols`; lines 321-324 are the silent drop). The missing check belongs in `lib/kestrel-analyze/src/decl/conformance_rules.rs`.
- **Confidence:** high.
- **Fix shape:** add a DeclCheck for Struct/Enum/Protocol/Extension that resolves each `Conformances` entry and reports "cannot find protocol" (E476-style) or "'X' is a struct, not a protocol" (mirroring E436).

## M22: `import Nope.Thing` is accepted
- **Root cause:** `process_import` resolves the module path and, if it is `None`, just `return`s. No diagnostic is raised anywhere. Selective `import A.(Nope)` items that match nothing are also dropped silently.
- **Location:** `lib/kestrel-name-res/src/scope.rs:228-238` (`process_import`).
- **Confidence:** high.
- **Fix shape:** add an analyzer (or an accumulate at name resolution) for an unresolved import module path or unmatched selective item.

## M23: `E.a.b` (case access on a case) type-checks as `()`
- **Root cause:** `E.a.b` lowers to `Field{ base: Def(case a), name: "b" }`, an instance-context member access (`is_static_context = false`). `solve_member` falls back to `resolve_static_member` when instance lookup fails (NotFound or IsStatic). So it finds the *static* enum case `b` on a *value* receiver and types it (as `()`) instead of erroring.
- **Location:** `lib/kestrel-type-infer/src/solver/member.rs:487-492` (NotFound → static fallback) and `member.rs:616-631` (IsStatic → static fallback), in `solve_member`. The static/instance check at `member.rs:1090-1106` covers only the opposite direction (an instance member in a static context).
- **Confidence:** medium-high.
- **Fix shape:** allow the static fallback only when `is_static_context`, or when the receiver expression is type-level. Otherwise report `MemberIsStatic`.

## M25: 10,000 `let`s take ~99 s (quadratic)
- **Measurement:** 4,000 lets took about 14 s. Samples landed in mono passes and in Cranelift.
- **Root cause:** two quadratic MIR passes over a single huge entry block:
  1. `copy_propagation::optimize_block`, at `lib/kestrel-mir/src/passes/copy_propagation.rs:84-85`. For every instruction it snapshots `frozen.keys().filter(count>0)` into a fresh set. Keys whose count has dropped to 0 are never removed, so this is O(insts × borrows-ever) time and memory.
  2. `copy_lifetime::owned_params::run`, at `lib/kestrel-mir/src/passes/copy_lifetime/owned_params.rs:44-54`. It does `while let Some(site) = find_site(whole body)`, rewriting one site per full rescan, and `rewrite_site` deletes from the instruction `Vec` (memcpy).
- **Confidence:** medium. The hot frames were sampled, not profiled exhaustively.
- **Fix shape:**
  - copy_propagation: keep only the live frozen set (remove keys when the count reaches 0), or query "frozen at i" via intervals.
  - owned_params: collect all sites in one scan and rewrite them back-to-front in one rebuild.

## M27: an unterminated block comment gives no error, only E618 pointing at a stdlib file
- **Root cause, part 1:** the lexer's block-comment callback returns `true` (a valid token) when it reaches EOF without `*/`, so the rest of the file silently becomes a comment. The comment says "Unclosed comment - bump to end".
  - Location: `lib/kestrel-lexer/src/lib.rs:145-147` (`parse_block_comment`).
- **Root cause, part 2:** E618's span picks the *last `DeclSpan` in the entire entity tree*, stdlib included. So it points at `lang/std/text/views.ks`.
  - Location: `lib/kestrel-analyze/src/compilation/entry_point.rs:250-263` (`entry_module_span` / `last_decl_span`).
- **Confidence:** high.
- **Fix shape:**
  - Return an error (or emit an "unterminated block comment" diagnostic) on EOF in `parse_block_comment`.
  - Anchor E618 on the user's entry file/module, not the global last decl.

---

## Other notes
- `return_return_value.ks` is H21; `nested_same_name.ks` is H2; `dup_field.ks` works correctly (E474).
- `unary_deep.ks` (L1) overflows in move_tracking, but its 512-deep variant is L5.
