# Root causes — codegen/ bugs (+ L8)
Base: /home/user/kestrel @ 3fea2041, prebuilt ./target/release/kestrel (not rebuilt). All repros re-run and reproduced.

## C2 — `arr(i).field.x = v` / `arr(i).0 = v` silently lost
- Cause: `lower_assign` only builds a get→modify→set writeback when the field's *immediate* base is an accessor (`arr(0).h`). For `arr(0).i.x`, base=`arr(0).i` is a stored field over an accessor: `try_field_addr_chain` fails (only Var-rooted), `try_lower_field_assign_through_setter` fails (stored field `i` has no setter → `fabricate_setter_writeback_slot` returns None at `find_setter_child(member)?`), so it falls to the rvalue fallback that mutates a temp copy. TupleIndex arm has no setter path at all.
- Loc: lib/kestrel-mir-lower/src/body/expr.rs:1283-1294 (Field arm fallback) and :1300-1313 (TupleIndex fallback) in `lower_assign`; lib/kestrel-mir-lower/src/body/place.rs:374 `try_field_addr_chain`; lib/kestrel-mir-lower/src/body/mod.rs:3989 (`fabricate_setter_writeback_slot`), :4083 `try_lower_field_assign_through_setter`.
- MIR (raw): `struct_extract %v34,.0; copy_value; begin_mut_borrow; field_addr; store_init` into a copy that is then destroyed — no `subscript.set`.
- Confidence: high.
- Fix: make the place walker accessor-aware — when a Field/TupleIndex chain bottoms out on a subscript/computed member, fabricate one writeback slot at that member and return FieldAddr projections off the slot (one shared path for Field and TupleIndex, also for compound `+=`).

## H18 — `@main func main(x: Int64)` → OSSA ICE
- Cause: EntryPointAnalyzer validates only free-function-ness (E615), return type (E616), multiplicity (E617), missing (E618) — never parameter count. MIR wrapper then calls user main with `vec![]` args.
- Loc: lib/kestrel-analyze/src/compilation/entry_point.rs:112-121 (`check`); wrapper lib/kestrel-mir-lower/src/items/static_lower.rs:87 `synthesize_main_wrapper` (push_call with no args).
- Confidence: high. Fix: add an E6xx "@main must take no parameters" check next to E616.

## H20 — `Int8(from: Int8)` type-checks then fails post-mono verify
- Cause: 7 concrete `init(from:)` overloads match labels → resolver's multi-match collapse `try_resolve_through_protocol` resolves to the abstract `Convertible.init(from:)` (From = wildcard), so arg type Int8 binds From=Int8. Applicability then checks only `Int8: Convertible` (protocol entity, no args), which holds; `Int8: Convertible[Int8]` is never checked. MIR shows `call @witness Convertible.init(from:) for Int8[Int8,Int8]`; post-mono verify finds no witness.
- Loc: lib/kestrel-type-infer/src/resolve.rs:667-679 (Ambiguous arm) + :1642 `try_resolve_through_protocol`; lib/kestrel-type-infer/src/solver/conformance.rs:56-57 (`ApplicableSource::Protocol` → `kind_conformance(recv, proto)` ignores protocol type args).
- Confidence: high. Fix: when collapsing through a parameterized protocol, emit a full conformance obligation `Recv: Proto[args]` (with From solved from the call) so it errors at type-check (or pick concrete overload after args concrete via `types_compatible`).

## H26 — `Float64.round()` ties-to-even
- Cause: LLVM backend lowers `FloatMathKind::Round` to `llvm.roundeven`; Cranelift to `nearest` (also ties-to-even). Docs (float64.ks:524-536) promise ties away from zero.
- Loc: lib/kestrel-codegen-llvm/src/inst.rs:810; lib/kestrel-codegen-cranelift/src/inst.rs:692. Intrinsic `f64_round`/`f32_round` mapped at lib/kestrel-mir-lower/src/body/call/intrinsic.rs:1190,1215.
- Confidence: high. Fix: LLVM `llvm.round`; Cranelift: emit `trunc(x + copysign(0.5-ulp…))` / `copysign(floor(|x|+0.5), x)` sequence (no native op).

## M18 — `t.1.0` rejected
- Cause: lexer greedily tokenizes `1.0` as `Float` (regex `[0-9][0-9_]*\.[0-9][0-9_]*…`), so after `.` the parser's `member` sees Float, not Integer, and reports E "expected identifier after `.`".
- Loc: lib/kestrel-lexer/src/lib.rs:222 (Float regex); lib/kestrel-parser/src/grammar/exprs.rs:325-333 (`member` only accepts `K::Integer`).
- Confidence: high. Fix: in `member`, accept a `Float` token after `.` whose text is `INT.INT` (no exponent) and split it into two ExprTupleIndex nodes (Swift/Rust approach), or lex `Float` not after a `.` token.

## M26 — `(-0.0).abs()` returns -0 (stdlib)
- Loc: lang/std/numeric/float64.ks:502-504 `abs()` — `if f64_lt(self, 0.0) { negate } else { self }`; `-0.0 < 0.0` is false, so -0.0 returned unchanged (doc line 500 says sign bit cleared).
- Confidence: high. Fix: clear the sign bit via bits (`f64_from_bits(to_bits & 0x7FFF…)`) or `llvm.fabs` intrinsic. (Check float32.ks for the same pattern.)

## L9 — `:.N` on floats switches to scientific (stdlib)
- Cause: Auto style uses `%g` semantics: with explicit precision it goes scientific when `decExp >= precision`, and precision is then significant digits — `12.34:.1`→`1.2e1`, `2.5:.0`→`2e0`, `999.9999:.2`→`1.00e3`. Docs (format.ks:226,363; float64.ks:1104) say decimal places.
- Loc: lang/std/numeric/float64.ks:1196-1200 (`if style == .Auto { if m == 0 or (decExp >= -4 and decExp < precision) {.Fixed} else {sci} }`) in `Float64.format`; field doc format.ks:243 even documents the sig-digit behaviour → docs inconsistent.
- Confidence: high. Fix: when `precisionProvided`, map Auto → Fixed (precision = decimal places); keep %g only if docs are changed instead.

## L10 — `Int64.minValue.gcd(6)` = -2 (stdlib)
- Loc: lang/std/numeric/int64.ks:614-623 `gcd` — `self.abs()` on minValue wraps to minValue (abs at :459 uses wrapping negate), Euclid then yields a negative result. Same pattern likely in int8/16/32.
- Confidence: high. Fix: run Euclid on unsigned magnitudes (`UInt64` of `abs` via `0 &- x`) and convert back (result 2^63 only for gcd(min,min/0) — trap or document).

## L8 — Bool width / float hex specifiers ignored (stdlib)
- Bool: lang/std/core/bool.ks:136-145 `Bool.format` appends "true"/"false" directly, never calls `_writePadded` (format.ks:412), so width/align/fill dropped. Fix: route through `_writePadded(into: writer, content: value, options)`.
- Float `:x`: lang/std/numeric/float64.ks:1110 `format` never reads `options.radix` — this is documented (format.ks:209, 354: "radix only applies to integers"), so not a stdlib bug per se; real gap is the frontend accepting `x` on a non-integer without a diagnostic (interpolation spec parsing). Confidence: high (Bool), medium on how to classify hex.
